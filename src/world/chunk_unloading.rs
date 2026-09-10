use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::split_dimension_position, lighting::relight_after_chunk_unloads,
        neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
};

use super::{
    chunk_remesh::ChunkRemeshQueue,
    chunk_system_params::{ChunkContent, ChunkRenderer},
    render_distance::{RenderDistanceSettings, chunk_is_in_volume},
};

const MAX_CHUNK_UNLOADS_PER_FRAME: usize = 2;

pub fn unload_chunk_meshes(
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut world: ResMut<VoxelWorld>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = render_distance.chunks();
    let vertical_radius = render_distance.vertical_chunks();
    let mut to_unload = renderer
        .pool
        .active_coords()
        .filter(|coord| !chunk_is_in_volume(center, *coord, horizontal_radius, vertical_radius))
        .collect::<Vec<_>>();

    to_unload.sort_by_key(|coord| -(*coord - center).length_squared());
    to_unload.truncate(MAX_CHUNK_UNLOADS_PER_FRAME);

    for coord in &to_unload {
        let Some((entities, mesh_handles)) = renderer.pool.take(*coord) else {
            continue;
        };

        for mesh_handle in mesh_handles {
            let _ = renderer.meshes.remove(&mesh_handle);
        }

        for entity in entities {
            renderer.commands.entity(entity).despawn();
        }

        world.archive_chunk(*coord);
    }

    if to_unload.is_empty() {
        return;
    }

    let lighting_changes =
        relight_after_chunk_unloads(&mut world, &to_unload, &content.blocks, &content.fluids);
    remesh_queue.extend(lighting_changes);

    for coord in &to_unload {
        for offset in CARDINAL_NEIGHBORS {
            remesh_queue.enqueue(*coord + offset);
        }
    }
}
