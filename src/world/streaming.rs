mod selection;
mod surface_cache;

use std::{
    collections::{HashMap, HashSet, VecDeque},
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
        lighting::{PendingLightingUpdates, seed_chunk_direct_lighting},
        neighbors::CARDINAL_NEIGHBORS,
        world::VoxelWorld,
    },
};

use self::selection::rebuild_queue;
use super::{
    biome_field::BiomeField,
    chunk_loading::ensure_chunk_loaded,
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::{ChunkRenderPool, spawn_chunk_mesh},
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::PendingFluidUpdates,
    render_distance::RenderDistanceSettings,
    work_budget::FrameWorkBudget,
    world_feature_fields::WorldFeatureFields,
};

const MIN_CHUNKS_BEFORE_BUDGET_CHECK: usize = 1;
const MAX_CHUNKS_PER_FRAME: usize = 4;
const STREAMING_BUDGET: Duration = Duration::from_millis(4);

#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_radius: i32,
    vertical_radius: i32,
    desired: HashSet<IVec3>,
    retained: HashSet<IVec3>,
    pending: VecDeque<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
}

impl ChunkStreamingState {
    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
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

    let generation_context = generation.context(&content);
    let mut budget = FrameWorkBudget::new(STREAMING_BUDGET, MIN_CHUNKS_BEFORE_BUDGET_CHECK)
        .with_maximum_items(MAX_CHUNKS_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = runtime.state.pending.pop_front() else {
            break;
        };

        if renderer.pool.contains(coord) {
            continue;
        }

        ensure_chunk_loaded(&mut runtime.world, coord, &generation_context);
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

        let chunk = runtime
            .world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        let chunk_is_empty = chunk.is_empty();
        let render_context = content.render_context(
            &runtime.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );

        spawn_chunk_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            chunk,
            &render_context,
        );
        budget.record(1);

        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if !renderer.pool.contains(neighbor) {
                continue;
            }

            if chunk_is_empty {
                runtime.remesh_queue.enqueue_fluid_priority(neighbor);
                continue;
            }

            let Some(neighbor_chunk) = runtime.world.chunk(neighbor) else {
                continue;
            };
            let chunk_boundary_has_content = boundary_has_content(chunk, offset);
            let neighbor_boundary_has_content = boundary_has_content(neighbor_chunk, -offset);

            if chunk_boundary_has_content && neighbor_boundary_has_content {
                runtime.remesh_queue.enqueue_priority(neighbor);
            } else if boundary_has_fluid(chunk, offset)
                || boundary_has_fluid(neighbor_chunk, -offset)
            {
                runtime.remesh_queue.enqueue_fluid_priority(neighbor);
            }
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
