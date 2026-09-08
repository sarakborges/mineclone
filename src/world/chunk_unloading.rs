use bevy::prelude::*;

use crate::{
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    voxel::coordinates::split_dimension_position,
};

use super::{
    chunk_rendering::RenderedChunk,
    render_distance::RenderDistanceSettings,
};

pub fn unload_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    rendered_chunks: Query<(Entity, &RenderedChunk, Option<&Mesh3d>)>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let radius = render_distance.chunks();
    let radius_squared = radius * radius;

    for (entity, rendered_chunk, mesh) in &rendered_chunks {
        let dx = rendered_chunk.coord.x - player_chunk.x;
        let dz = rendered_chunk.coord.z - player_chunk.z;

        if dx * dx + dz * dz <= radius_squared {
            continue;
        }

        if let Some(mesh) = mesh {
            let _ = meshes.remove(mesh.id());
        }

        commands.entity(entity).despawn();
    }
}
