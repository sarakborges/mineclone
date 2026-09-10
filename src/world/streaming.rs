use std::collections::VecDeque;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::split_dimension_position, lighting::initialize_chunk_lighting,
        neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
};

use super::{
    chunk_loading::ensure_chunk_loaded,
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::{ChunkRenderPool, spawn_chunk_mesh},
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    render_distance::{RenderDistanceSettings, chunk_coords_in_volume},
};

const CHUNKS_PER_FRAME: usize = 2;

#[derive(Resource, Default)]
pub struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_render_distance: i32,
    vertical_render_distance: i32,
    pending: VecDeque<IVec3>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingInputs<'w, 's> {
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    render_distance: Res<'w, RenderDistanceSettings>,
    world: ResMut<'w, VoxelWorld>,
    streaming: ResMut<'w, ChunkStreamingState>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub fn reset_chunk_streaming(mut state: ResMut<ChunkStreamingState>) {
    *state = ChunkStreamingState::default();
}

pub fn stream_chunks(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut inputs: ChunkStreamingInputs,
) {
    let feet_position = inputs.player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = inputs.render_distance.chunks();
    let vertical_radius = inputs.render_distance.vertical_chunks();

    if inputs.streaming.center != Some(center)
        || inputs.streaming.horizontal_render_distance != horizontal_radius
        || inputs.streaming.vertical_render_distance != vertical_radius
    {
        rebuild_queue(
            &mut inputs.streaming,
            &renderer.pool,
            center,
            horizontal_radius,
            vertical_radius,
        );
    }

    let generation_context = generation.context(&content);

    for _ in 0..CHUNKS_PER_FRAME {
        let Some(coord) = inputs.streaming.pending.pop_front() else {
            break;
        };

        if renderer.pool.contains(coord) {
            continue;
        }

        ensure_chunk_loaded(&mut inputs.world, coord, &generation_context);
        let lighting_changes =
            initialize_chunk_lighting(&mut inputs.world, coord, &content.blocks, &content.fluids);
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

        inputs.remesh_queue.extend(lighting_changes);
        for offset in CARDINAL_NEIGHBORS {
            inputs.remesh_queue.enqueue(coord + offset);
        }
    }
}

fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    render_pool: &ChunkRenderPool,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
) {
    let coords = chunk_coords_in_volume(center, horizontal_radius, vertical_radius);

    streaming.center = Some(center);
    streaming.horizontal_render_distance = horizontal_radius;
    streaming.vertical_render_distance = vertical_radius;
    streaming.pending = coords
        .into_iter()
        .filter(|coord| !render_pool.contains(*coord))
        .collect();
}
