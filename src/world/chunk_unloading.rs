use bevy::prelude::*;

use crate::{
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    voxel::coordinates::split_dimension_position,
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
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let radius = render_distance.chunks();
    let radius_squared = radius * radius;
    let to_unload = render_pool
        .active_coords()
        .filter(|coord| {
            let dx = coord.x - player_chunk.x;
            let dz = coord.z - player_chunk.z;
            dx * dx + dz * dz > radius_squared
        })
        .collect::<Vec<_>>();

    for coord in to_unload {
        let Some((entity, mesh_handle)) = render_pool.take(coord) else {
            continue;
        };

        if let Some(mesh_handle) = mesh_handle {
            let _ = meshes.remove(&mesh_handle);
            render_pool.recycle_mesh_handle(mesh_handle);
        }

        commands.entity(entity).despawn();
    }
}
