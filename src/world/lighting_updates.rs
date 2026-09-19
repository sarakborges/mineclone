use std::time::Duration;

use bevy::{
    ecs::system::SystemParam,
    platform::collections::HashSet,
    prelude::*,
};

use crate::voxel::{
    chunk::VoxelChunk,
    lighting::{PendingLightingUpdates, process_pending_lighting},
    world::VoxelWorld,
};

use super::{
    chunk_remesh::ChunkRemeshQueue, chunk_remesh_tasks::ChunkRemeshTasks,
    chunk_system_params::VoxelContent, work_budget::FrameWorkBudget,
};

const LIGHTING_BUDGET: Duration = Duration::from_millis(2);
const MIN_LIGHTING_VOXELS_BEFORE_BUDGET_CHECK: usize = 256;
const MAX_LIGHTING_VOXELS_PER_FRAME: usize = 4_096;

#[derive(SystemParam)]
pub(super) struct DynamicLightingRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
    remesh_tasks: ResMut<'w, ChunkRemeshTasks>,
}

pub(super) fn pending_lighting_work(lighting: Res<PendingLightingUpdates>) -> bool {
    !lighting.is_empty()
}

pub(super) fn process_dynamic_lighting(
    content: VoxelContent,
    mut changed_chunks: Local<HashSet<IVec3>>,
    mut runtime: DynamicLightingRuntime,
) {
    if runtime.lighting.is_empty() {
        return;
    }

    let mut budget = FrameWorkBudget::new(LIGHTING_BUDGET, MIN_LIGHTING_VOXELS_BEFORE_BUDGET_CHECK)
        .with_maximum_items(MAX_LIGHTING_VOXELS_PER_FRAME);
    let mut recorded_voxels = 0;
    process_pending_lighting(
        &mut runtime.world,
        &mut runtime.lighting,
        &content.blocks,
        &content.fluids,
        &content.secondary_properties,
        &mut changed_chunks,
        |processed_voxels| {
            budget.record(processed_voxels.saturating_sub(recorded_voxels));
            recorded_voxels = processed_voxels;
            budget.exhausted()
        },
    );

    runtime
        .remesh_tasks
        .bump_lighting_revisions(changed_chunks.iter().copied());

    for coord in changed_chunks.drain() {
        if runtime.world.chunk(coord).is_none() {
            continue;
        }
        enqueue_lighting_remesh(coord, &mut runtime);
    }
}

fn enqueue_lighting_remesh(
    coord: IVec3,
    runtime: &mut DynamicLightingRuntime<'_>,
) {
    runtime
        .remesh_queue
        .enqueue_lighting_change(coord, &runtime.world);

    // Face lighting/AO samples the complete 3x3x3 one-voxel halo. Publish
    // revisions immediately; async remesh dependency checks reject stale
    // captures if lighting changes again while the task is in flight.
    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if x.abs() + y.abs() + z.abs() <= 1 {
                    continue;
                }
                let neighbor = coord + offset;
                if neighbor.y < 0 {
                    continue;
                }
                let Some(chunk) = runtime.world.chunk(neighbor) else {
                    continue;
                };
                let needs_geometry = diagonal_boundary_has_content(chunk, offset);
                let needs_fluid = diagonal_boundary_has_fluid(chunk, offset);
                if needs_geometry {
                    runtime.remesh_queue.enqueue_priority(neighbor);
                }
                if needs_fluid {
                    runtime.remesh_queue.enqueue_fluid_priority(neighbor);
                }
            }
        }
    }
}

fn diagonal_boundary_has_content(chunk: &VoxelChunk, offset: IVec3) -> bool {
    (offset.x == 0 || chunk.boundary_has_content(IVec3::new(-offset.x, 0, 0)))
        && (offset.y == 0 || chunk.boundary_has_content(IVec3::new(0, -offset.y, 0)))
        && (offset.z == 0 || chunk.boundary_has_content(IVec3::new(0, 0, -offset.z)))
}

fn diagonal_boundary_has_fluid(chunk: &VoxelChunk, offset: IVec3) -> bool {
    (offset.x == 0 || chunk.boundary_has_fluid(IVec3::new(-offset.x, 0, 0)))
        && (offset.y == 0 || chunk.boundary_has_fluid(IVec3::new(0, -offset.y, 0)))
        && (offset.z == 0 || chunk.boundary_has_fluid(IVec3::new(0, 0, -offset.z)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{cell::VoxelCell, texture_rotation::TextureRotation};

    #[test]
    fn diagonal_invalidation_requires_content_on_each_toward_source_boundary() {
        let mut chunk = VoxelChunk::empty();
        chunk.set_block(
            0,
            0,
            7,
            Some(VoxelCell::new("asteria:test", TextureRotation::default())),
        );

        assert!(diagonal_boundary_has_content(&chunk, IVec3::new(1, 1, 0)));
        assert!(!diagonal_boundary_has_content(&chunk, IVec3::new(1, 1, 1)));
        assert!(!diagonal_boundary_has_content(&chunk, IVec3::new(-1, 1, 0)));
        assert!(!diagonal_boundary_has_fluid(&chunk, IVec3::new(1, 1, 0)));
    }
}
