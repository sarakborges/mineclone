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
        coordinates::chunk_coord_from_position, lighting::PendingLightingUpdates,
        neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
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
const STREAMING_LIGHT_BATCH_CHUNKS: usize = 1;
const STREAMING_BUDGET: Duration = Duration::from_millis(5);

#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_radius: i32,
    vertical_radius: i32,
    desired: HashSet<IVec3>,
    pending: VecDeque<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
}

impl ChunkStreamingState {
    pub(super) fn wants(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord)
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
        if processed >= MIN_CHUNKS_BEFORE_BUDGET_CHECK
            && frame_started.elapsed() >= STREAMING_BUDGET
        {
            break;
        }

        let mut batch = Vec::with_capacity(STREAMING_LIGHT_BATCH_CHUNKS);

        while batch.len() < STREAMING_LIGHT_BATCH_CHUNKS {
            if processed + batch.len() >= MIN_CHUNKS_BEFORE_BUDGET_CHECK
                && frame_started.elapsed() >= STREAMING_BUDGET
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
            batch.push(coord);
        }

        if batch.is_empty() {
            break;
        }

        // Streaming only seeds the lighting queue. The bounded PostUpdate lighting
        // pass performs propagation, preventing one newly generated chunk from
        // monopolizing a frame while still allowing the chunk mesh to appear now.
        lighting_updates.enqueue_chunks_initialization(&mut inputs.world, &batch);

        for &coord in &batch {
            let chunk = inputs
                .world
                .chunk(coord)
                .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
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
        }

        processed += batch.len();

        for &coord in &batch {
            for offset in CARDINAL_NEIGHBORS {
                let neighbor = coord + offset;
                if !batch.contains(&neighbor) && renderer.pool.contains(neighbor) {
                    inputs.remesh_queue.enqueue_priority(neighbor);
                }
            }
        }
    }
}
