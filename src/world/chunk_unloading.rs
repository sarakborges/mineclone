use std::time::Instant;

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

const CHUNK_UNLOAD_BUDGET_MS: u128 = 2;

pub(super) fn unload_chunk_meshes(
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
    let mut pending_unloads = renderer
        .pool
        .active_coords()
        .filter(|coord| !streaming.wants(*coord))
        .collect::<Vec<_>>();

    pending_unloads.sort_by_key(|coord| -(*coord - center).length_squared());

    let frame_started = Instant::now();
    let mut unloaded = Vec::new();

    for coord in pending_unloads {
        if !unloaded.is_empty() && frame_started.elapsed().as_millis() >= CHUNK_UNLOAD_BUDGET_MS {
            break;
        }

        let Some((entities, mesh_handles)) = renderer.pool.take(coord) else {
            continue;
        };

        for mesh_handle in mesh_handles {
            let _ = renderer.meshes.remove(&mesh_handle);
        }

        for entity in entities {
            renderer.commands.entity(entity).despawn();
        }

        world.archive_chunk(coord);
        unloaded.push(coord);
    }

    if unloaded.is_empty() {
        return;
    }

    lighting.enqueue_chunk_unloads(&unloaded);

    for coord in unloaded {
        for offset in CARDINAL_NEIGHBORS {
            remesh_queue.enqueue(coord + offset);
        }
    }
}
