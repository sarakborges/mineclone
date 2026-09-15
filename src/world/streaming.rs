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
const MAX_GENERATION_DISPATCH_WORK_PER_FRAME: usize = 4;
const MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME: usize = 8;
const MAX_MESH_RESULTS_COLLECTED_PER_FRAME: usize = 4;
const GENERATION_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
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
    retired: DeduplicatedQueue<IVec3>,
    pending: DeduplicatedQueue<IVec3>,
    ready: DeduplicatedQueue<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
}

impl ChunkStreamingState {
    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
    }

    pub(super) fn enqueue_retired(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.retired.enqueue(coord);
        }
    }

    pub(super) fn pop_retired(&mut self) -> Option<IVec3> {
        self.retired.pop()
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
pub(super) struct ChunkStreamingWork<'w> {
    world: ResMut<'w, VoxelWorld>,
    state: ResMut<'w, ChunkStreamingState>,
    generation_tasks: ResMut<'w, ChunkGenerationTasks>,
    mesh_tasks: ResMut<'w, ChunkMeshTasks>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingSelection<'w, 's> {
    render_distance: Res<'w, RenderDistanceSettings>,
    scratch: Local<'s, selection::QueueRebuildScratch>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingQueues<'w> {
    remesh: ResMut<'w, ChunkRemeshQueue>,
    fluid: ResMut<'w, PendingFluidUpdates>,
    lighting: ResMut<'w, PendingLightingUpdates>,
}

pub(super) fn stream_chunks(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    player: Single<&Transform, With<GameplayCamera>>,
    mut selection: ChunkStreamingSelection,
    mut work: ChunkStreamingWork,
    mut queues: ChunkStreamingQueues,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = selection.render_distance.chunks();
    let vertical_radius = selection.render_distance.vertical_chunks();

    if work.state.center != Some(center)
        || work.state.horizontal_radius != horizontal_radius
        || work.state.vertical_radius != vertical_radius
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
            &mut work.state,
            center,
            horizontal_radius,
            vertical_radius,
            &mut selection.scratch,
            &rebuild_context,
        );
    }

    work.generation_tasks.sync_snapshot(&generation, &content);
    work.mesh_tasks.sync_snapshot(&content);

    if work.generation_tasks.pending_count() > 0 {
        collect_generated_chunks(&mut work);
    }
    if work.mesh_tasks.pending_count() > 0 {
        collect_built_chunk_meshes(&content, &mut renderer, &mut work, &mut queues.remesh);
    }
    if work.generation_tasks.pending_count() < MAX_GENERATION_TASKS_IN_FLIGHT
        && work.state.pending.len() > 0
    {
        dispatch_generation_tasks(&renderer.pool, &mut work);
    }
    if work.state.ready.len() > 0 {
        dispatch_initial_mesh_tasks(&content, &mut renderer, &mut work, &mut queues);
    }
}

fn collect_generated_chunks(work: &mut ChunkStreamingWork<'_>) {
    let current_revision = work.generation_tasks.revision();
    let mut budget = FrameWorkBudget::new(GENERATION_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = work.generation_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            work.state.requeue(completed.coord);
            continue;
        }
        if !work.state.keeps_loaded(completed.coord) {
            continue;
        }
        if work.world.has_generated_chunk(completed.coord) {
            work.state.mark_ready(completed.coord);
            continue;
        }

        work.world.insert_chunk(completed.coord, completed.output);
        work.state.mark_ready(completed.coord);
    }
}

fn dispatch_generation_tasks(render_pool: &ChunkRenderPool, work: &mut ChunkStreamingWork<'_>) {
    let mut budget = FrameWorkBudget::new(GENERATION_DISPATCH_BUDGET, 1)
        .with_maximum_items(MAX_GENERATION_DISPATCH_WORK_PER_FRAME);

    while work.generation_tasks.pending_count() < MAX_GENERATION_TASKS_IN_FLIGHT {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = work.state.pending.pop() else {
            break;
        };

        if render_pool.contains(coord)
            || work.state.ready.contains(coord)
            || work.mesh_tasks.contains(coord)
        {
            continue;
        }
        if work.generation_tasks.contains(coord) {
            continue;
        }

        if work.world.has_generated_chunk(coord) {
            assert!(
                work.world.restore_chunk(coord),
                "generated chunk must be resident or archived: {coord:?}"
            );
            work.state.mark_ready(coord);
            budget.record(1);
            continue;
        }

        if work.generation_tasks.schedule(coord) {
            budget.record(1);
        } else {
            work.state.requeue(coord);
            break;
        }
    }
}

fn dispatch_initial_mesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
) {
    let mut budget = FrameWorkBudget::new(STREAMING_BUDGET, MIN_CHUNKS_BEFORE_BUDGET_CHECK)
        .with_maximum_items(MAX_CHUNKS_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = work.state.ready.pop() else {
            break;
        };
        if !work.state.keeps_loaded(coord) || renderer.pool.contains(coord) {
            continue;
        }
        if work.mesh_tasks.contains(coord) {
            continue;
        }
        let Some(chunk_is_empty) = work.world.chunk(coord).map(|chunk| chunk.is_empty()) else {
            work.state.requeue(coord);
            continue;
        };
        if !chunk_is_empty && work.mesh_tasks.pending_count() >= MAX_MESH_TASKS_IN_FLIGHT {
            work.state.defer_ready(coord);
            break;
        }

        queues
            .fluid
            .enqueue_loaded_fluid_frontier(&work.world, coord);
        seed_chunk_direct_lighting(
            &mut work.world,
            coord,
            content.blocks(),
            content.fluids(),
            content.secondary_properties(),
        );
        if chunk_is_empty {
            queues.lighting.enqueue_empty_chunk_relaxation(coord);
        } else {
            queues.lighting.enqueue_chunk_relaxation(coord);
        }

        if chunk_is_empty {
            integrate_empty_chunk(content, renderer, &work.world, &mut queues.remesh, coord);
            budget.record(1);
            continue;
        }

        let snapshot = ChunkMeshSnapshot::capture(&work.world, coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        if !work.mesh_tasks.schedule(coord, snapshot) {
            work.state.defer_ready(coord);
            break;
        }

        budget.record(1);
    }
}

fn integrate_empty_chunk(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    world: &VoxelWorld,
    remesh_queue: &mut ChunkRemeshQueue,
    coord: IVec3,
) {
    let render_context = content.render_context(
        world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );
    spawn_built_chunk_meshes(
        &mut renderer.commands,
        &mut renderer.meshes,
        &mut renderer.pool,
        coord,
        Vec::new(),
        &render_context,
    );
    notify_loaded_chunk_neighbors(coord, true, world, &renderer.pool, remesh_queue);
}

fn collect_built_chunk_meshes(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    work: &mut ChunkStreamingWork<'_>,
    remesh_queue: &mut ChunkRemeshQueue,
) {
    let current_revision = work.mesh_tasks.revision();
    let mut budget = FrameWorkBudget::new(MESH_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_MESH_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = work.mesh_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        if completed.revision != current_revision {
            work.state.mark_ready(completed.coord);
            continue;
        }
        if !work.state.keeps_loaded(completed.coord) || renderer.pool.contains(completed.coord) {
            continue;
        }
        if !completed.output.dependencies.is_current(&work.world) {
            work.state.mark_ready(completed.coord);
            continue;
        }
        let Some(chunk) = work.world.chunk(completed.coord) else {
            work.state.requeue(completed.coord);
            continue;
        };
        let chunk_is_empty = chunk.is_empty();
        let render_context = content.render_context(
            &work.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );

        spawn_built_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            completed.coord,
            completed.output.meshes,
            &render_context,
        );
        notify_loaded_chunk_neighbors(
            completed.coord,
            chunk_is_empty,
            &work.world,
            &renderer.pool,
            remesh_queue,
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
        let Some(neighbor_chunk) = world.chunk(neighbor) else {
            continue;
        };

        if chunk_is_empty {
            if neighbor_chunk.boundary_has_fluid(-offset) {
                remesh_queue.enqueue_fluid_priority(neighbor);
            }
            continue;
        }

        let chunk_boundary_has_content = chunk.boundary_has_content(offset);
        let neighbor_boundary_has_content = neighbor_chunk.boundary_has_content(-offset);

        if chunk_boundary_has_content && neighbor_boundary_has_content {
            remesh_queue.enqueue_priority(neighbor);
        } else if chunk.boundary_has_fluid(offset) || neighbor_chunk.boundary_has_fluid(-offset) {
            remesh_queue.enqueue_fluid_priority(neighbor);
        }
    }
}
