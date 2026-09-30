use std::time::Duration;

use crate::app::crash_log::log_gameplay_event;

use bevy::{
    ecs::system::SystemParam,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::voxel::{
    coordinates::visit_chunk_coords_whose_voxel_halo_contains,
    lighting::{PendingLightingUpdates, process_pending_lighting},
    meshlet::ChunkMeshletMask,
    world::VoxelWorld,
};

use super::{
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::ChunkRenderPool,
    chunk_system_params::VoxelContent,
    presentation_snapshot::PresentationLightingRevisions,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
};

const LIGHTING_BUDGET: Duration = Duration::from_millis(2);
const MIN_LIGHTING_VOXELS_BEFORE_BUDGET_CHECK: usize = 256;
const MAX_LIGHTING_VOXELS_PER_FRAME: usize = 4_096;
#[derive(Default)]struct LightingDiagnostics {    timer: Option<Timer>,    processed_voxels: u64,}
#[derive(SystemParam)]
pub(super) struct DynamicLightingRuntime<'w, 's> {
    world: ResMut<'w, VoxelWorld>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
    lighting_revisions: ResMut<'w, PresentationLightingRevisions>,
    frame_budget: Res<'w, WorldFrameWorkBudget>,
    render_pool: Res<'w, ChunkRenderPool>,
    changed_chunks: Local<'s, HashSet<IVec3>>,
    changed_positions: Local<'s, HashSet<IVec3>>,
    dirty_meshlets: Local<'s, HashMap<IVec3, ChunkMeshletMask>>,
    diagnostics: Local<'s, LightingDiagnostics>,
    time: Res<'w, Time<Real>>,
}

pub(super) fn pending_lighting_work(lighting: Res<PendingLightingUpdates>) -> bool {
    !lighting.is_empty()
}

pub(super) fn process_dynamic_lighting(
    content: VoxelContent,
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
        &mut runtime.changed_chunks,
        &mut runtime.changed_positions,
        &|coord| runtime.render_pool.contains(coord),
        |processed_voxels| {
            budget.record(processed_voxels.saturating_sub(recorded_voxels));
            recorded_voxels = processed_voxels;
            budget.exhausted()
        },
    );

    let changed_chunk_count = runtime.changed_chunks.len();
    let changed_position_count = runtime.changed_positions.len();
    runtime.diagnostics.processed_voxels = runtime
        .diagnostics
        .processed_voxels
        .saturating_add(recorded_voxels as u64);

    runtime.dirty_meshlets.clear();
    for position in runtime.changed_positions.drain() {
        visit_chunk_coords_whose_voxel_halo_contains(position, |coord| {
            if runtime.world.chunk(coord).is_none() {
                return;
            }

            let meshlets = ChunkMeshletMask::for_world_position(coord, position);
            let dirty = runtime.dirty_meshlets.entry(coord).or_default();
            *dirty = dirty.union(meshlets);
        });
    }

    let dirty_meshlet_count = dirty_meshlets.len();
    let dirty_meshlet_count = runtime.dirty_meshlets.len();
    for (&coord, &meshlets) in runtime.dirty_meshlets.iter() {
        runtime.lighting_revisions.bump(coord, meshlets);
    }
    changed_chunks.clear();

    for (coord, meshlets) in runtime.dirty_meshlets.drain() {
        runtime
            .remesh_queue
            .enqueue_lighting_meshlet_change(coord, meshlets, &runtime.world);
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

    let timer = runtime
        .diagnostics
        .timer
        .get_or_insert_with(|| Timer::from_seconds(5.0, TimerMode::Repeating));
    timer.tick(runtime.time.delta());
    if timer.just_finished() {
        log_gameplay_event(format!(
            "world.lighting.runtime processed_voxels={} changed_chunks={} changed_positions={} dirty_meshlets={} propagation_pending={}",
            runtime.diagnostics.processed_voxels,
            changed_chunk_count,
            changed_position_count,
            dirty_meshlet_count,
            runtime.lighting.has_propagation_work(),
        ));
        runtime.diagnostics.processed_voxels = 0;
    }
}


