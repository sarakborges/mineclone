use bevy::{platform::collections::HashSet, prelude::*};

use crate::{
    voxel::lighting::{PendingLightingUpdates, process_pending_lighting},
    world::{chunk_system_params::ChunkContent, work_budget::FrameWorkBudget},
};

use super::{INITIAL_LOADING_BUDGET, super::{WorldLoadingPhase, system_params::WorldSetupProgress}};

pub(super) fn light_initial_chunks(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    lighting: &mut PendingLightingUpdates,
    changed_chunks: &mut HashSet<IVec3>,
) {
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 256);

    loop {
        if budget.exhausted() {
            break;
        }

        if lighting.is_empty() {
            let Some(coord) = progress
                .loading_state
                .coords
                .get(progress.loading_state.lit)
                .copied()
            else {
                progress.loading_state.phase = WorldLoadingPhase::Meshing;
                break;
            };
            assert!(
                lighting.enqueue_initial_chunk_lighting(&mut progress.world, coord),
                "generated bootstrap chunk data should exist at {coord:?}"
            );
        }

        let mut recorded_voxels = 0;
        process_pending_lighting(
            &mut progress.world,
            lighting,
            content.blocks(),
            content.fluids(),
            content.secondary_properties(),
            changed_chunks,
            |processed_voxels| {
                budget.record(processed_voxels.saturating_sub(recorded_voxels));
                recorded_voxels = processed_voxels;
                budget.exhausted()
            },
        );
        changed_chunks.clear();

        if !lighting.is_empty() {
            break;
        }

        progress.loading_state.lit += 1;
    }

    if progress.loading_state.lit >= progress.loading_state.coords.len() && lighting.is_empty() {
        progress.loading_state.phase = WorldLoadingPhase::Meshing;
    }
}
