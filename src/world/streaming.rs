mod selection;
mod surface_cache;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
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
pub(super) struct ChunkStreamingInputs<'w, 's> {
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    render_distance: Res<'w, RenderDistanceSettings>,
    world: ResMut<'w, VoxelWorld>,
    streaming: ResMut<'w, ChunkStreamingState>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub(super) fn reset_chunk_streaming(mut state: ResMut<ChunkStreamingState>) {
    *state = ChunkStreamingState::default();
}

pub(super) fn stream_chunks(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut inputs: ChunkStreamingInputs,
    mut fluid_updates: ResMut<PendingFluidUpdates>,
    mut lighting_updates: ResMut<PendingLightingUpdates>,
) {
    let feet_position = inputs.player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = inputs.render_distance.chunks();
    let vertical_radius = inputs.render_distance.vertical_chunks();

    if inputs.streaming.center != Some(center)
        || inputs.streaming.horizontal_radius != horizontal_radius
        || inputs.streaming.vertical_radius != vertical_radius
    {
        let rebuild_context = QueueRebuildContext {
            render_pool: &renderer.pool,
            dimension: generation.dimension(),
            biomes: &content.biomes,
            structures: &content.structures,
            biome_field: &content.biome_field,
            feature_fields: &generation.feature_fields,
        };
        rebuild_queue(
            &mut inputs.streaming,
            center,
            horizontal_radius,
            vertical_radius,
            &rebuild_context,
        );
    }

    let generation_context = generation.context(&content);
    let frame_started = Instant::now();
    let mut processed = 0;

    loop {
        if processed >= MAX_CHUNKS_PER_FRAME
            || (processed >= MIN_CHUNKS_BEFORE_BUDGET_CHECK
                && frame_started.elapsed() >= STREAMING_BUDGET)
        {
            break;
        }

        let Some(coord) = inputs.streaming.pending.pop_front() else {
            break;
        };

        if renderer.pool.contains(coord) {
            continue;
        }

        ensure_chunk_loaded(&mut inputs.world, coord, &generation_context);
        fluid_updates.enqueue_loaded_fluid_frontier(&inputs.world, coord);

        seed_chunk_direct_lighting(
            &mut inputs.world,
            coord,
            &content.blocks,
            &content.fluids,
            &content.secondary_properties,
        );
        lighting_updates.enqueue_chunk_relaxation(coord);

        let chunk = inputs
            .world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        let chunk_is_empty = chunk.is_empty();
        let render_context = content.render_context(
            &inputs.world,
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
        processed += 1;

        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if !renderer.pool.contains(neighbor) {
                continue;
            }

            if chunk_is_empty {
                inputs.remesh_queue.enqueue_fluid_priority(neighbor);
                continue;
            }

            let Some(neighbor_chunk) = inputs.world.chunk(neighbor) else {
                continue;
            };
            if boundary_has_content(chunk, offset)
                && boundary_has_content(neighbor_chunk, -offset)
            {
                inputs.remesh_queue.enqueue_priority(neighbor);
            }
        }
    }
}

fn boundary_has_content(chunk: &VoxelChunk, outward: IVec3) -> bool {
    let last = CHUNK_SIZE as i32 - 1;

    match outward {
        IVec3::X => (0..CHUNK_SIZE as i32).any(|y| {
            (0..CHUNK_SIZE as i32).any(|z| voxel_has_content(chunk, last, y, z))
        }),
        IVec3::NEG_X => (0..CHUNK_SIZE as i32).any(|y| {
            (0..CHUNK_SIZE as i32).any(|z| voxel_has_content(chunk, 0, y, z))
        }),
        IVec3::Y => (0..CHUNK_SIZE as i32).any(|z| {
            (0..CHUNK_SIZE as i32).any(|x| voxel_has_content(chunk, x, last, z))
        }),
        IVec3::NEG_Y => (0..CHUNK_SIZE as i32).any(|z| {
            (0..CHUNK_SIZE as i32).any(|x| voxel_has_content(chunk, x, 0, z))
        }),
        IVec3::Z => (0..CHUNK_SIZE as i32).any(|y| {
            (0..CHUNK_SIZE as i32).any(|x| voxel_has_content(chunk, x, y, last))
        }),
        IVec3::NEG_Z => (0..CHUNK_SIZE as i32).any(|y| {
            (0..CHUNK_SIZE as i32).any(|x| voxel_has_content(chunk, x, y, 0))
        }),
        _ => false,
    }
}

fn voxel_has_content(chunk: &VoxelChunk, x: i32, y: i32, z: i32) -> bool {
    chunk.cell_at(x, y, z).is_some() || chunk.fluid_at(x, y, z).is_some()
}
