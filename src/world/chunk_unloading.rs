use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::split_dimension_position, lighting::PendingLightingUpdates,
        neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
};

use super::{
    chunk_remesh::ChunkRemeshQueue,
    chunk_system_params::ChunkRenderer,
    streaming::ChunkStreamingState,
};

const MAX_CHUNK_UNLOADS_PER_FRAME: usize = 2;

pub fn unload_chunk_meshes(
    player: Single<&Transform, With<GameplayCamera>>,
    streaming: Res<ChunkStreamingState>,
    mut renderer: ChunkRenderer,
    mut world: ResMut<VoxelWorld>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let mut to_unload = renderer
        .pool
        .active_coords()
        .filter(|coord| !streaming.wants(*coord))
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

    lighting.enqueue_chunk_unloads(&to_unload);

    for coord in &to_unload {
        for offset in CARDINAL_NEIGHBORS {
            remesh_queue.enqueue(*coord + offset);
        }
    }
}
