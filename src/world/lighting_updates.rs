use std::time::Duration;

use bevy::{
    ecs::system::SystemParam,
    platform::collections::HashSet,
    prelude::*,
};

use crate::voxel::{
    lighting::{PendingLightingUpdates, process_pending_lighting},
    world::VoxelWorld,
};

use super::{
    chunk_remesh::ChunkRemeshQueue, chunk_remesh_tasks::ChunkRemeshTasks,
    chunk_system_params::VoxelContent,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
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
    frame_budget: Res<'w, WorldFrameWorkBudget>,
}

pub(super) fn pending_lighting_work(lighting: Res<PendingLightingUpdates>) -> bool {
    !lighting.is_empty()
}

pub(super) fn process_dynamic_lighting(
    content: VoxelContent,
    mut changed_chunks: Local<HashSet<IVec3>>,
    mut changed_positions: Local<HashSet<IVec3>>,
    mut runtime: DynamicLightingRuntime,
) {
    if runtime.lighting.is_empty() {
        return;
    }

    let mut budget = FrameWorkBudget::new(LIGHTING_BUDGET, MIN_LIGHTING_VOXELS_BEFORE_BUDGET_CHECK)
        .with_global_deadline(runtime.frame_budget.deadline())
        .with_maximum_items(MAX_LIGHTING_VOXELS_PER_FRAME);
    let mut recorded_voxels = 0;
    process_pending_lighting(
        &mut runtime.world,
        &mut runtime.lighting,
        &content.blocks,
        &content.fluids,
        &content.secondary_properties,
        &mut changed_chunks,
        &mut changed_positions,
        |processed_voxels| {
            budget.record(processed_voxels.saturating_sub(recorded_voxels));
            recorded_voxels = processed_voxels;
            budget.exhausted()
        },
    );

    runtime
        .remesh_tasks
        .bump_lighting_revisions(changed_chunks.iter().copied());
    changed_chunks.clear();

    for position in changed_positions.drain() {
        runtime
            .remesh_queue
            .enqueue_lighting_voxel_change(position, &runtime.world);
    }

    if let Some((priority, background)) =
        runtime.lighting.take_completed_settling_fluid_remeshes()
    {
        for coord in priority {
            if runtime.world.chunk(coord).is_some() {
                runtime.remesh_queue.enqueue_fluid_priority(coord);
            }
        }
        for coord in background {
            if runtime.world.chunk(coord).is_some() {
                runtime.remesh_queue.enqueue_fluid(coord);
            }
        }
    }
}


