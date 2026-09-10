use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::split_dimension_position, lighting::initialize_chunk_lighting,
        world::VoxelWorld,
    },
};

use super::{
    chunk_loading::ensure_chunk_loaded,
    chunk_rendering::{
        ChunkRenderPool, refresh_adjacent_chunk_meshes, refresh_changed_chunk_meshes,
        spawn_chunk_mesh,
    },
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

pub fn reset_chunk_streaming(mut state: ResMut<ChunkStreamingState>) {
    *state = ChunkStreamingState::default();
}

pub fn stream_chunks(
    player: Single<&Transform, With<GameplayCamera>>,
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    render_distance: Res<RenderDistanceSettings>,
    mut world: ResMut<VoxelWorld>,
    mut streaming: ResMut<ChunkStreamingState>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = render_distance.chunks();
    let vertical_radius = render_distance.vertical_chunks();

    if streaming.center != Some(center)
        || streaming.horizontal_render_distance != horizontal_radius
        || streaming.vertical_render_distance != vertical_radius
    {
        rebuild_queue(
            &mut streaming,
            &renderer.pool,
            center,
            horizontal_radius,
            vertical_radius,
        );
    }

    let generation_context = generation.context(&content);

    for _ in 0..CHUNKS_PER_FRAME {
        let Some(coord) = streaming.pending.pop_front() else {
            break;
        };

        if renderer.pool.contains(coord) {
            continue;
        }

        ensure_chunk_loaded(&mut world, coord, &generation_context);
        let lighting_changes =
            initialize_chunk_lighting(&mut world, coord, &content.blocks, &content.fluids);
        let chunk = world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        let render_context = content.render_context(
            &world,
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
        refresh_adjacent_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            &render_context,
        );
        refresh_changed_chunk_meshes(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            lighting_changes,
            &render_context,
        );
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
