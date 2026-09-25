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
    let mut changed_positions = HashSet::new();

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

            let chunk_is_empty = progress
                .world
                .chunk(coord)
                .unwrap_or_else(|| {
                    panic!("generated bootstrap chunk data should exist at {coord:?}")
                })
                .is_empty();
            let direct_seed = lighting.seed_chunk_direct_lighting(
                &mut progress.world,
                coord,
                content.blocks(),
                content.fluids(),
                content.secondary_properties(),
            );

            if chunk_is_empty {
                lighting.enqueue_empty_chunk_relaxation(coord);
            } else if direct_seed.requires_relaxation {
                lighting.enqueue_chunk_relaxation(coord);
            }

            if lighting.is_empty() {
                progress.loading_state.lit += 1;
                continue;
            }
        }

        let mut recorded_voxels = 0;
        process_pending_lighting(
            &mut progress.world,
            lighting,
            content.blocks(),
            content.fluids(),
            content.secondary_properties(),
            changed_chunks,
            &mut changed_positions,
            |processed_voxels| {
                budget.record(processed_voxels.saturating_sub(recorded_voxels));
                recorded_voxels = processed_voxels;
                budget.exhausted()
            },
        );
        changed_chunks.clear();
        changed_positions.clear();

        if !lighting.is_empty() {
            break;
        }

        progress.loading_state.lit += 1;
    }

    if progress.loading_state.lit >= progress.loading_state.coords.len() && lighting.is_empty() {
        progress.loading_state.phase = WorldLoadingPhase::Meshing;
    }
}
