use std::time::Duration;

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
    chunk_rendering::retire_chunk_render_allocation,
    chunk_system_params::ChunkRenderer,
    render_distance::RenderDistanceSettings,
    streaming::ChunkStreamingState,
    work_budget::FrameWorkBudget,
};

const MIN_CHUNKS_BEFORE_UNLOAD_BUDGET_CHECK: usize = 1;
const CHUNK_UNLOAD_BUDGET: Duration = Duration::from_millis(4);
const MIN_UNLOAD_RETENTION_MARGIN_CHUNKS: i32 = 10;

#[derive(Resource, Default)]
pub(super) struct ChunkUnloadState {
    bootstrapped: bool,
}

impl ChunkUnloadState {
    fn bootstrap(
        &mut self,
        streaming: &mut ChunkStreamingState,
        world: &VoxelWorld,
        center: IVec3,
    ) {
        if self.bootstrapped {
            return;
        }

        let mut pending = world
            .loaded_chunk_coords()
            .filter(|coord| !streaming.keeps_loaded(*coord))
            .collect::<Vec<_>>();
        pending.sort_by_key(|coord| -(*coord - center).length_squared());
        for coord in pending {
            streaming.enqueue_retired(coord);
        }

        self.bootstrapped = true;
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
    render_distance: Res<RenderDistanceSettings>,
    mut streaming: ResMut<ChunkStreamingState>,
    mut renderer: ChunkRenderer,
    mut runtime: ChunkUnloadRuntime,
    mut unloaded: Local<Vec<IVec3>>,
) {
    unloaded.clear();

    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let retention_radius = unload_retention_radius(render_distance.chunks());
    runtime
        .state
        .bootstrap(&mut streaming, &runtime.world, center);

    let mut budget = FrameWorkBudget::new(
        CHUNK_UNLOAD_BUDGET,
        MIN_CHUNKS_BEFORE_UNLOAD_BUDGET_CHECK,
    );

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = streaming.pop_retired_outside_horizontal_radius(center, retention_radius)
        else {
            break;
        };
        if streaming.keeps_loaded(coord) || runtime.world.chunk(coord).is_none() {
            continue;
        }

        retire_chunk_render_allocation(&mut renderer.commands, &mut renderer.pool, coord);
        runtime.remesh_queue.remove(coord);
        runtime.world.archive_chunk(coord);
        unloaded.push(coord);
        budget.record(1);
    }

    if unloaded.is_empty() {
        return;
    }

    runtime
        .lighting
        .enqueue_chunk_unloads(unloaded.as_slice());

    for coord in unloaded.drain(..) {
        for offset in CARDINAL_NEIGHBORS {
            runtime.remesh_queue.enqueue_priority(coord + offset);
        }
    }
}

fn unload_retention_radius(render_distance_chunks: i32) -> i32 {
    let nominal_radius = render_distance_chunks.max(1);
    let proportional_margin = (nominal_radius + 1) / 2;
    nominal_radius.saturating_add(proportional_margin.max(MIN_UNLOAD_RETENTION_MARGIN_CHUNKS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unload_retention_scales_from_render_distance() {
        assert_eq!(unload_retention_radius(4), 14);
        assert_eq!(unload_retention_radius(12), 22);
        assert_eq!(unload_retention_radius(24), 36);
    }
}
