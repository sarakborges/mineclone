mod selection;
mod surface_cache;

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        biome::BiomeRegistry, dimension::DimensionDefinition, structure::StructureRegistry,
    },
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        coordinates::chunk_coord_from_position,
        deduplicated_queue::DeduplicatedQueue,
        lighting::{PendingLightingUpdates, seed_chunk_direct_lighting},
        mesh_snapshot::ChunkMeshSnapshot,
        neighbors::CARDINAL_NEIGHBORS,
        world::VoxelWorld,
    },
};

use self::selection::rebuild_queue;
use super::{
    biome_field::BiomeField,
    chunk_generation_tasks::{ChunkGenerationTasks, MAX_GENERATION_TASKS_IN_FLIGHT},
    chunk_mesh_tasks::{ChunkMeshTasks, MAX_MESH_TASKS_IN_FLIGHT},
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::{ChunkRenderPool, spawn_built_chunk_meshes},
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::PendingFluidUpdates,
    render_distance::RenderDistanceSettings,
    work_budget::FrameWorkBudget,
    world_feature_fields::WorldFeatureFields,
};

const MIN_CHUNKS_BEFORE_BUDGET_CHECK: usize = 1;
const MAX_CHUNKS_PER_FRAME: usize = 4;
const MAX_GENERATION_TASKS_DISPATCHED_PER_FRAME: usize = 4;
const MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME: usize = 8;
const MAX_MESH_RESULTS_COLLECTED_PER_FRAME: usize = 4;
const GENERATION_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const MESH_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(2);
const STREAMING_BUDGET: Duration = Duration::from_millis(4);

#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_radius: i32,
    vertical_radius: i32,
    desired: HashSet<IVec3>,
    retained: HashSet<IVec3>,
    pending: DeduplicatedQueue<IVec3>,
    ready: DeduplicatedQueue<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
}

impl ChunkStreamingState {
    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
    }

    fn requeue(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.pending.contains(coord) && !self.ready.contains(coord) {
            self.pending.enqueue_front(coord);
        }
    }

    fn mark_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue(coord);
        }
    }

    fn defer_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue_front(coord);
        }
    }
}

struct QueueRebuildContext<'a> {
    render_pool: &'a ChunkRenderPool,
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
    structures: &'a StructureRegistry,
    biome_field: &'a BiomeField,
    feature_fields: &'a WorldFeatureFields,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingRuntime<'w, 's> {
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    render_distance: Res<'w, RenderDistanceSettings>,
    world: ResMut<'w, VoxelWorld>,
    state: ResMut<'w, ChunkStreamingState>,
    generation_tasks: ResMut<'w, ChunkGenerationTasks>,
    mesh_tasks: ResMut<'w, ChunkMeshTasks>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
    fluid_updates: ResMut<'w, PendingFluidUpdates>,
    lighting_updates: ResMut<'w, PendingLightingUpdates>,
}

pub(super) fn stream_chunks(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut runtime: ChunkStreamingRuntime,
) {
    let feet_position = runtime.player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = runtime.render_distance.chunks();
    let vertical_radius = runtime.render_distance.vertical_chunks();

    if runtime.state.center != Some(center)
        || runtime.state.horizontal_radius != horizontal_radius
        || runtime.state.vertical_radius != vertical_radius
    {
        let rebuild_context = QueueRebuildContext {
            render_pool: &renderer.pool,
            dimension: generation.dimension(),
            biomes: &content.biomes,
            structures: &generation.structures,
            biome_field: &content.biome_field,
            feature_fields: &generation.feature_fields,
        };
        rebuild_queue(
            &mut runtime.state,
            center,
            horizontal_radius,
            vertical_radius,
            &rebuild_context,
        );
    }

    runtime
        .generation_tasks
        .sync_snapshot(&generation, &content);
    runtime.mesh_tasks.sync_snapshot(&content);

    collect_generated_chunks(&mut runtime);
    collect_built_chunk_meshes(&content, &mut renderer, &mut runtime);
    dispatch_generation_tasks(&renderer.pool, &mut runtime);
    dispatch_initial_mesh_tasks(&content, &renderer.pool, &mut runtime);
}

fn collect_generated_chunks(runtime: &mut ChunkStreamingRuntime<'_, '_>) {
    let current_revision = runtime.generation_tasks.revision();
    let mut budget = FrameWorkBudget::new(GENERATION_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = runtime.generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            runtime.state.requeue(completed.coord);
            continue;
        }
        if !runtime.state.keeps_loaded(completed.coord) {
            continue;
        }
        if runtime.world.has_generated_chunk(completed.coord) {
            runtime.state.mark_ready(completed.coord);
            continue;
        }

        runtime.world.insert_chunk(completed.coord, completed.output);
        runtime.state.mark_ready(completed.coord);
    }
}

fn dispatch_generation_tasks(
    render_pool: &ChunkRenderPool,
    runtime: &mut ChunkStreamingRuntime<'_, '_>,
) {
    let mut dispatched = 0;

    while runtime.generation_tasks.pending_count() < MAX_GENERATION_TASKS_IN_FLIGHT
        && dispatched < MAX_GENERATION_TASKS_DISPATCHED_PER_FRAME
    {
        let Some(coord) = runtime.state.pending.pop() else {
            break;
        };

        if render_pool.contains(coord)
            || runtime.state.ready.contains(coord)
            || runtime.mesh_tasks.contains(coord)
        {
            continue;
        }
        if runtime.generation_tasks.contains(coord) {
            continue;
        }

        if runtime.world.has_generated_chunk(coord) {
            assert!(
                runtime.world.restore_chunk(coord),
                "generated chunk must be resident or archived: {coord:?}"
            );
            runtime.state.mark_ready(coord);
            continue;
        }

        if runtime.generation_tasks.schedule(coord) {
            dispatched += 1;
        } else {
            runtime.state.requeue(coord);
            break;
        }
    }
}

fn dispatch_initial_mesh_tasks(
    content: &ChunkContent<'_>,
    render_pool: &ChunkRenderPool,
    runtime: &mut ChunkStreamingRuntime<'_, '_>,
) {
    let mut budget = FrameWorkBudget::new(STREAMING_BUDGET, MIN_CHUNKS_BEFORE_BUDGET_CHECK)
        .with_maximum_items(MAX_CHUNKS_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = runtime.state.ready.pop() else {
            break;
        };
        if !runtime.state.keeps_loaded(coord) || render_pool.contains(coord) {
            continue;
        }
        if runtime.mesh_tasks.contains(coord) {
            continue;
        }
        if runtime.mesh_tasks.pending_count() >= MAX_MESH_TASKS_IN_FLIGHT {
            runtime.state.defer_ready(coord);
            break;
        }
        if runtime.world.chunk(coord).is_none() {
            runtime.state.requeue(coord);
            continue;
        }

        runtime
            .fluid_updates
            .enqueue_loaded_fluid_frontier(&runtime.world, coord);
        seed_chunk_direct_lighting(
            &mut runtime.world,
            coord,
            content.blocks(),
            content.fluids(),
            content.secondary_properties(),
        );
        runtime.lighting_updates.enqueue_chunk_relaxation(coord);

        let snapshot = ChunkMeshSnapshot::capture(&runtime.world, coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        if !runtime.mesh_tasks.schedule(coord, snapshot) {
            runtime.state.defer_ready(coord);
            break;
        }

        budget.record(1);
    }
}

fn collect_built_chunk_meshes(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    runtime: &mut ChunkStreamingRuntime<'_, '_>,
) {
    let current_revision = runtime.mesh_tasks.revision();
    let mut budget = FrameWorkBudget::new(MESH_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_MESH_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = runtime.mesh_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            runtime.state.mark_ready(completed.coord);
            continue;
        }
        if !runtime.state.keeps_loaded(completed.coord) || renderer.pool.contains(completed.coord) {
            continue;
        }
        let Some(chunk) = runtime.world.chunk(completed.coord) else {
            runtime.state.requeue(completed.coord);
            continue;
        };
        let chunk_is_empty = chunk.is_empty();
        let render_context = content.render_context(
            &runtime.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );

        spawn_built_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            completed.coord,
            completed.output,
            &render_context,
        );
        notify_loaded_chunk_neighbors(
            completed.coord,
            chunk_is_empty,
            &runtime.world,
            &renderer.pool,
            &mut runtime.remesh_queue,
        );
    }
}

fn notify_loaded_chunk_neighbors(
    coord: IVec3,
    chunk_is_empty: bool,
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    remesh_queue: &mut ChunkRemeshQueue,
) {
    let chunk = world
        .chunk(coord)
        .unwrap_or_else(|| panic!("rendered chunk data should exist at {coord:?}"));

    for offset in CARDINAL_NEIGHBORS {
        let neighbor = coord + offset;
        if !render_pool.contains(neighbor) {
            continue;
        }

        if chunk_is_empty {
            remesh_queue.enqueue_fluid_priority(neighbor);
            continue;
        }

        let Some(neighbor_chunk) = world.chunk(neighbor) else {
            continue;
        };
        let chunk_boundary_has_content = boundary_has_content(chunk, offset);
        let neighbor_boundary_has_content = boundary_has_content(neighbor_chunk, -offset);

        if chunk_boundary_has_content && neighbor_boundary_has_content {
            remesh_queue.enqueue_priority(neighbor);
        } else if boundary_has_fluid(chunk, offset) || boundary_has_fluid(neighbor_chunk, -offset) {
            remesh_queue.enqueue_fluid_priority(neighbor);
        }
    }
}

fn boundary_has_content(chunk: &VoxelChunk, outward: IVec3) -> bool {
    boundary_any(chunk, outward, |chunk, x, y, z| {
        chunk.cell_at(x, y, z).is_some() || chunk.fluid_at(x, y, z).is_some()
    })
}

fn boundary_has_fluid(chunk: &VoxelChunk, outward: IVec3) -> bool {
    boundary_any(chunk, outward, |chunk, x, y, z| {
        chunk.fluid_at(x, y, z).is_some()
    })
}

fn boundary_any(
    chunk: &VoxelChunk,
    outward: IVec3,
    mut predicate: impl FnMut(&VoxelChunk, i32, i32, i32) -> bool,
) -> bool {
    let size = CHUNK_SIZE as i32;
    let last = size - 1;

    if outward.x != 0 {
        let x = if outward.x > 0 { last } else { 0 };
        return (0..size).any(|y| (0..size).any(|z| predicate(chunk, x, y, z)));
    }
    if outward.y != 0 {
        let y = if outward.y > 0 { last } else { 0 };
        return (0..size).any(|z| (0..size).any(|x| predicate(chunk, x, y, z)));
    }
    if outward.z != 0 {
        let z = if outward.z > 0 { last } else { 0 };
        return (0..size).any(|y| (0..size).any(|x| predicate(chunk, x, y, z)));
    }

    false
}
