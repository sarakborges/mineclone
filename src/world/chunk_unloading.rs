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
    chunk_rendering::{
        CHUNK_MESH_RESIDENCY_HIGH_BYTES, CHUNK_MESH_RESIDENCY_RECOVERY_BYTES,
        CHUNK_MESH_RESIDENCY_TARGET_BYTES, ChunkRenderPool, retire_chunk_render_allocation,
    },
    chunk_system_params::ChunkRenderer,
    render_distance::RenderDistanceSettings,
    streaming::ChunkStreamingState,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
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
            .filter(|coord| !runtime.streaming.keeps_loaded(*coord))
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
    frame_budget: Res<'w, WorldFrameWorkBudget>,
}

#[derive(SystemParam)]
pub(super) struct ChunkMeshResidencyRuntime<'w> {
    streaming: ResMut<'w, ChunkStreamingState>,
    world: Res<'w, VoxelWorld>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
    remesh_tasks: ResMut<'w, ChunkRemeshTasks>,
}

pub(super) fn retire_distant_chunk_meshes(
    mut renderer: ChunkRenderer,
    streaming: Res<ChunkStreamingState>,
    world: Res<VoxelWorld>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
    mut remesh_tasks: ResMut<ChunkRemeshTasks>,
    mut retired: Local<Vec<IVec3>>,
) {
    retired.clear();
    retired.extend(
        renderer
            .pool
            .active_coords()
            .filter(|coord| !streaming.retains_render_mesh(*coord)),
    );
    if retired.is_empty() {
        return;
    }

    retired.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
    for coord in retired.drain(..) {
        retire_chunk_render_allocation(&mut renderer.commands, &mut renderer.pool, coord);
        remesh_queue.remove(coord);
        remesh_tasks.remove_lighting_revision(coord);
        enqueue_retired_render_halo_remeshes(
            coord,
            &runtime.world,
            &renderer.pool,
            &mut runtime.remesh_queue,
        );
    }
}

#[derive(Clone, Copy)]
pub(super) struct MeshResidencyCandidate {
    coord: IVec3,
    bytes: usize,
    visible: bool,
    critical: bool,
    horizontal_distance_squared: i64,
    total_distance_squared: i64,
}

pub(super) fn enforce_chunk_mesh_residency_budget(
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    mut renderer: ChunkRenderer,
    mut runtime: ChunkMeshResidencyRuntime,
    mut candidates: Local<Vec<MeshResidencyCandidate>>,
    mut recovery: Local<Vec<IVec3>>,
) {
    let before = renderer.pool.mesh_bytes();
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let center = chunk_coord_from_position(feet_position);

    if before <= CHUNK_MESH_RESIDENCY_RECOVERY_BYTES {
        recovery.clear();
        recovery.extend(
            runtime.streaming
                .mesh_pressure_evicted_coords()
                .filter(|coord| runtime.streaming.keeps_loaded(*coord) && !renderer.pool.contains(*coord)),
        );
        recovery.sort_unstable_by_key(|coord| {
            let delta = *coord - center;
            (
                delta.length_squared(),
                coord.y,
                coord.z,
                coord.x,
            )
        });
        for coord in recovery.iter().copied().take(2) {
            runtime.streaming.recover_mesh_after_pressure(coord);
        }
    }

    if before <= CHUNK_MESH_RESIDENCY_HIGH_BYTES {
        return;
    }

    let visible_radius = i64::from(render_distance.chunks().max(1));
    let visible_radius_squared = visible_radius * visible_radius;

    candidates.clear();
    candidates.extend(renderer.pool.active_coords().filter_map(|coord| {
        let bytes = renderer.pool.mesh_bytes_for(coord);
        if bytes == 0 {
            return None;
        }

        let dx = i64::from(coord.x) - i64::from(center.x);
        let dy = i64::from(coord.y) - i64::from(center.y);
        let dz = i64::from(coord.z) - i64::from(center.z);
        let horizontal_distance_squared =
            dx.saturating_mul(dx).saturating_add(dz.saturating_mul(dz));
        let total_distance_squared = horizontal_distance_squared
            .saturating_add(dy.saturating_mul(dy));

        Some(MeshResidencyCandidate {
            coord,
            bytes,
            visible: horizontal_distance_squared <= visible_radius_squared,
            critical: dx.abs() <= 1 && dy.abs() <= 1 && dz.abs() <= 1,
            horizontal_distance_squared,
            total_distance_squared,
        })
    }));

    // Evict hysteresis/preload allocations first, then the farthest visible
    // allocations. The 3x3x3 player neighborhood is the last resort, not an
    // absolute exemption: preventing a render OOM is more important than
    // retaining any individual chunk mesh.
    candidates.sort_unstable_by(|left, right| {
        left.visible
            .cmp(&right.visible)
            .then_with(|| left.critical.cmp(&right.critical))
            .then_with(|| {
                right
                    .total_distance_squared
                    .cmp(&left.total_distance_squared)
            })
            .then_with(|| {
                right
                    .horizontal_distance_squared
                    .cmp(&left.horizontal_distance_squared)
            })
            .then_with(|| right.bytes.cmp(&left.bytes))
            .then_with(|| left.coord.y.cmp(&right.coord.y))
            .then_with(|| left.coord.z.cmp(&right.coord.z))
            .then_with(|| left.coord.x.cmp(&right.coord.x))
    });

    let mut resident_bytes = before;
    let mut evicted_chunks = 0_usize;
    let mut evicted_bytes = 0_usize;

    for candidate in candidates.iter().copied() {
        if resident_bytes <= CHUNK_MESH_RESIDENCY_TARGET_BYTES {
            break;
        }
        if !renderer.pool.contains(candidate.coord) {
            continue;
        }

        retire_chunk_render_allocation(
            &mut renderer.commands,
            &mut renderer.pool,
            candidate.coord,
        );
        runtime.streaming.suppress_mesh_for_pressure(candidate.coord);
        runtime.remesh_queue.remove(candidate.coord);
        runtime.remesh_tasks.remove_lighting_revision(candidate.coord);
        enqueue_retired_render_halo_remeshes(
            candidate.coord,
            &world,
            &renderer.pool,
            &mut remesh_queue,
        );

        resident_bytes = resident_bytes.saturating_sub(candidate.bytes);
        evicted_bytes = evicted_bytes.saturating_add(candidate.bytes);
        evicted_chunks += 1;
    }

    if evicted_chunks > 0 {
        warn!(
            "chunk mesh residency pressure: before_bytes={before} after_bytes={} high_watermark_bytes={} target_bytes={} evicted_chunks={evicted_chunks} evicted_bytes={evicted_bytes}",
            renderer.pool.mesh_bytes(),
            CHUNK_MESH_RESIDENCY_HIGH_BYTES,
            CHUNK_MESH_RESIDENCY_TARGET_BYTES,
        );
    }
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
    )
    .with_global_deadline(runtime.frame_budget.deadline());

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(coord) = streaming.pop_retired_outside_horizontal_radius(center, retention_radius)
        else {
            break;
        };
        if streaming.keeps_loaded(coord)
            || streaming.generated_chunk_is_unpublished(coord)
            || runtime.world.chunk(coord).is_none()
        {
            continue;
        }

        retire_chunk_render_allocation(&mut renderer.commands, &mut renderer.pool, coord);
        runtime.remesh_queue.remove(coord);
        runtime.remesh_tasks.remove_lighting_revision(coord);
        runtime.world.archive_chunk(coord);

        // Removing a 16³ section can reopen direct skylight for every
        // resident section below it in the same x/z column.
        runtime
            .lighting
            .enqueue_loaded_column_below(&runtime.world, coord);

        // Restored or newly generated chunks need a fresh direct-light seed,
        // but an obsolete mesh retry while still resident must not reseed.
        streaming.forget_initial_lighting_seeded(coord);
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
        enqueue_retired_render_halo_remeshes(
            coord,
            &runtime.world,
            &renderer.pool,
            &mut runtime.remesh_queue,
        );
    }
}

// Vertex lighting/AO and fluid corner heights use all 26 rendered neighbors,
// not just the six cardinals. Retiring a render allocation must invalidate
// neighboring mesh families even when the source chunk remains resident as
// preload/cache data. Check only rendered neighbors with actual content on each
// toward-source face.
fn enqueue_retired_render_halo_remeshes(
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
    use crate::voxel::{
        cell::VoxelCell, fluid::FluidCell, texture_rotation::TextureRotation,
    };

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

        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(1, 1, 0)), (true, false));
        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(-1, 1, 0)), (false, false));
        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(1, 1, 1)), (false, false));
    }

    #[test]
    fn fluid_halo_remesh_is_independent_of_terrain_content() {
        let mut chunk = VoxelChunk::empty();
        chunk.set_fluid(0, 0, 7, Some(FluidCell::source(0, 8)));

        // The generic content boundary counts include fluid occupancy too.
        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(1, 1, 0)), (true, true));
        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(1, 0, 0)), (true, true));
        assert_eq!(halo_remesh_needs(&chunk, IVec3::new(-1, 1, 0)), (false, false));
    }
}
