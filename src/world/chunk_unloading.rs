use std::time::{Duration, Instant};

use bevy::{ecs::system::SystemParam, prelude::*};

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

const MIN_CHUNKS_BEFORE_UNLOAD_BUDGET_CHECK: usize = 8;
const CHUNK_UNLOAD_BUDGET: Duration = Duration::from_millis(4);

#[derive(SystemParam)]
struct ChunkUnloadRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub(super) fn unload_chunk_meshes(
    player: Single<&Transform, With<GameplayCamera>>,
    streaming: Res<ChunkStreamingState>,
    mut renderer: ChunkRenderer,
    mut runtime: ChunkUnloadRuntime,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let mut pending_unloads = renderer
        .pool
        .active_coords()
        .filter(|coord| !streaming.keeps_loaded(*coord))
        .collect::<Vec<_>>();

    pending_unloads.sort_by_key(|coord| -(*coord - center).length_squared());

    let frame_started = Instant::now();
    let mut unloaded = Vec::new();

    for coord in pending_unloads {
        if unloaded.len() >= MIN_CHUNKS_BEFORE_UNLOAD_BUDGET_CHECK
            && frame_started.elapsed() >= CHUNK_UNLOAD_BUDGET
        {
            break;
        }

        let Some((entities, mesh_handles)) = renderer.pool.take(coord) else {
            continue;
        };

        for entity in entities {
            renderer.commands.entity(entity).despawn();
        }

        if !mesh_handles.is_empty() {
            renderer.commands.queue(move |world: &mut World| {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                for mesh_handle in mesh_handles {
                    let _ = meshes.remove(&mesh_handle);
                }
            });
        }

        runtime.world.archive_chunk(coord);
        unloaded.push(coord);
    }

    if unloaded.is_empty() {
        return;
    }

    runtime.lighting.enqueue_chunk_unloads(&unloaded);

    for coord in unloaded {
        for offset in CARDINAL_NEIGHBORS {
            runtime.remesh_queue.enqueue_priority(coord + offset);
        }
    }
}
