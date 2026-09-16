use std::time::Duration;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        chunk::VoxelChunk, coordinates::chunk_coord_from_position,
        lighting::PendingLightingUpdates, world::VoxelWorld,
    },
};

use super::{
    chunk_remesh::ChunkRemeshQueue,
    chunk_remesh_tasks::ChunkRemeshTasks,
    chunk_rendering::{ChunkRenderPool, retire_chunk_render_allocation},
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
    remesh_tasks: ResMut<'w, ChunkRemeshTasks>,
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

    let mut budget =
        FrameWorkBudget::new(CHUNK_UNLOAD_BUDGET, MIN_CHUNKS_BEFORE_UNLOAD_BUDGET_CHECK);

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
        runtime.remesh_tasks.remove_lighting_revision(coord);
        runtime.world.archive_chunk(coord);
        // Restored or newly generated chunks need a fresh direct-light seed,
        // but an obsolete mesh retry while still resident must not reseed.
        streaming.forget_initial_lighting_seeded(coord);
        unloaded.push(coord);
        budget.record(1);
    }

    if unloaded.is_empty() {
        return;
    }

    runtime.lighting.enqueue_chunk_unloads(unloaded.as_slice());

    for coord in unloaded.drain(..) {
        enqueue_unloaded_halo_remeshes(
            coord,
            &runtime.world,
            &renderer.pool,
            &mut runtime.remesh_queue,
        );
    }
}

// Vertex lighting/AO and fluid corner heights use all 26 neighbors, not just
// the six cardinals. An unloaded halo must invalidate both mesh families; a
// lighting relaxation is not guaranteed to change a voxel and trigger remesh.
// Check only rendered neighbors with actual content on each toward-source face.
fn enqueue_unloaded_halo_remeshes(
    coord: IVec3,
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    remesh_queue: &mut ChunkRemeshQueue,
) {
    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if offset == IVec3::ZERO {
                    continue;
                }
                let neighbor = coord + offset;
                if neighbor.y < 0 || !render_pool.contains(neighbor) {
                    continue;
                }
                let Some(chunk) = world.chunk(neighbor) else {
                    continue;
                };
                let (geometry, fluid) = halo_remesh_needs(chunk, offset);
                if geometry {
                    remesh_queue.enqueue_priority(neighbor);
                }
                if fluid {
                    remesh_queue.enqueue_fluid_priority(neighbor);
                }
            }
        }
    }
}

fn halo_remesh_needs(chunk: &VoxelChunk, offset: IVec3) -> (bool, bool) {
    let toward_faces = |has_face: fn(&VoxelChunk, IVec3) -> bool| {
        (offset.x == 0 || has_face(chunk, IVec3::new(-offset.x, 0, 0)))
            && (offset.y == 0 || has_face(chunk, IVec3::new(0, -offset.y, 0)))
            && (offset.z == 0 || has_face(chunk, IVec3::new(0, 0, -offset.z)))
    };
    (
        toward_faces(VoxelChunk::boundary_has_content),
        toward_faces(VoxelChunk::boundary_has_fluid),
    )
}

fn unload_retention_radius(render_distance_chunks: i32) -> i32 {
    let nominal_radius = render_distance_chunks.max(1);
    let proportional_margin = (nominal_radius + 1) / 2;
    nominal_radius.saturating_add(proportional_margin.max(MIN_UNLOAD_RETENTION_MARGIN_CHUNKS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{cell::VoxelCell, fluid::FluidCell, texture_rotation::TextureRotation};

    #[test]
    fn unload_retention_scales_from_render_distance() {
        assert_eq!(unload_retention_radius(4), 14);
        assert_eq!(unload_retention_radius(12), 22);
        assert_eq!(unload_retention_radius(24), 36);
    }

    #[test]
    fn diagonal_unload_invalidates_only_toward_source_content() {
        let mut chunk = VoxelChunk::empty();
        chunk.set_block(
            0,
            0,
            7,
            Some(VoxelCell::new("asteria:test", TextureRotation::default())),
        );

        assert_eq!(
            halo_remesh_needs(&chunk, IVec3::new(1, 1, 0)),
            (true, false)
        );
        assert_eq!(
            halo_remesh_needs(&chunk, IVec3::new(-1, 1, 0)),
            (false, false)
        );
        assert_eq!(
            halo_remesh_needs(&chunk, IVec3::new(1, 1, 1)),
            (false, false)
        );
    }

    #[test]
    fn fluid_halo_remesh_is_independent_of_terrain_content() {
        let mut chunk = VoxelChunk::empty();
        chunk.set_fluid(0, 0, 7, Some(FluidCell::source(0, 8)));

        // The generic content boundary counts include fluid occupancy too.
        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(1, 1, 0)), (true, true));
        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(1, 0, 0)), (true, true));
        assert_eq!(
            halo_remesh_needs(&chunk, IVec3::new(-1, 1, 0)),
            (false, false)
        );
    }
}
