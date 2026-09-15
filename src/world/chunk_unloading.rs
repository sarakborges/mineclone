use std::time::Duration;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::chunk_coord_from_position, deduplicated_queue::DeduplicatedQueue,
        lighting::PendingLightingUpdates, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
};

use super::{
    chunk_remesh::ChunkRemeshQueue,
    chunk_system_params::ChunkRenderer,
    streaming::ChunkStreamingState,
    work_budget::FrameWorkBudget,
};

const MIN_CHUNKS_BEFORE_UNLOAD_BUDGET_CHECK: usize = 8;
const CHUNK_UNLOAD_BUDGET: Duration = Duration::from_millis(4);

#[derive(Resource, Default)]
pub(super) struct ChunkUnloadState {
    selection_key: Option<(IVec3, i32, i32)>,
    pending: DeduplicatedQueue<IVec3>,
}

impl ChunkUnloadState {
    fn sync_plan(
        &mut self,
        streaming: &ChunkStreamingState,
        world: &VoxelWorld,
        center: IVec3,
    ) {
        let selection_key = streaming.selection_key();
        if self.selection_key == selection_key {
            return;
        }

        let mut pending = world
            .loaded_chunk_coords()
            .filter(|coord| !streaming.keeps_loaded(*coord))
            .collect::<Vec<_>>();
        pending.sort_by_key(|coord| -(*coord - center).length_squared());

        self.selection_key = selection_key;
        self.pending = pending.into_iter().collect();
    }
}

#[derive(SystemParam)]
pub(super) struct ChunkUnloadRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    state: ResMut<'w, ChunkUnloadState>,
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
    runtime
        .state
        .sync_plan(&streaming, &runtime.world, center);

    let mut budget = FrameWorkBudget::new(
        CHUNK_UNLOAD_BUDGET,
        MIN_CHUNKS_BEFORE_UNLOAD_BUDGET_CHECK,
    );
    let mut unloaded = Vec::new();

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = runtime.state.pending.pop() else {
            break;
        };
        if streaming.keeps_loaded(coord) || runtime.world.chunk(coord).is_none() {
            continue;
        }

        if let Some((entities, mesh_handles)) = renderer.pool.take(coord) {
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
        }

        runtime.world.archive_chunk(coord);
        unloaded.push(coord);
        budget.record(1);
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
