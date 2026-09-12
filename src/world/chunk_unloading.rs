use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::chunk_coord_from_position, lighting::PendingLightingUpdates,
        neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
};

use super::{
    chunk_remesh::ChunkRemeshQueue,
    chunk_system_params::ChunkRenderer,
    streaming::ChunkStreamingState,
};

pub(super) fn unload_chunk_meshes(
    player: Single<&Transform, With<GameplayCamera>>,
    streaming: Res<ChunkStreamingState>,
    mut renderer: ChunkRenderer,
    mut world: ResMut<VoxelWorld>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let mut pending_unloads = renderer
        .pool
        .active_coords()
        .filter(|coord| !streaming.wants(*coord))
        .collect::<Vec<_>>();

    // Render correctness takes priority over amortizing despawns. Leaving unwanted
    // render slots alive for several frames produces visible "ghost chunks" while
    // the player moves or the desired streaming set changes. Remove every stale
    // slot immediately; chunk generation remains independently budgeted.
    pending_unloads.sort_by_key(|coord| -(*coord - center).length_squared());

    let mut unloaded = Vec::with_capacity(pending_unloads.len());

    for coord in pending_unloads {
        let Some((entities, mesh_handles)) = renderer.pool.take(coord) else {
            continue;
        };

        for entity in entities {
            renderer.commands.entity(entity).despawn();
        }
        for mesh_handle in mesh_handles {
            let _ = renderer.meshes.remove(&mesh_handle);
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
            remesh_queue.enqueue_priority(coord + offset);
        }
    }
}
