use bevy::prelude::*;

use crate::{
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    voxel::{coordinates::split_dimension_position, world::VoxelWorld},
};

use super::{
    chunk_rendering::ChunkRenderPool,
    render_distance::RenderDistanceSettings,
};

pub fn unload_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    mut world: ResMut<VoxelWorld>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = render_distance.chunks();
    let vertical_radius = render_distance.vertical_chunks();
    let horizontal_radius_squared = horizontal_radius * horizontal_radius;
    let to_unload = render_pool
        .active_coords()
        .filter(|coord| {
            let delta = *coord - center;
            let outside_horizontal =
                delta.x * delta.x + delta.z * delta.z > horizontal_radius_squared;
            let outside_vertical = delta.y.abs() > vertical_radius;

            outside_horizontal || outside_vertical
        })
        .collect::<Vec<_>>();

    for coord in to_unload {
        let Some((entities, mesh_handles)) = render_pool.take(coord) else {
            continue;
        };

        for mesh_handle in mesh_handles {
            let _ = meshes.remove(&mesh_handle);
        }

        for entity in entities {
            commands.entity(entity).despawn();
        }

        world.archive_chunk(coord);
    }
}
