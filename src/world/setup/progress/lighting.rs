use bevy::{platform::collections::HashSet, prelude::*};

use crate::{
    voxel::lighting::{PendingLightingUpdates, process_pending_lighting},
    world::{chunk_system_params::ChunkContent, work_budget::FrameWorkBudget},
};

use super::{
    INITIAL_LOADING_BUDGET,
    super::{
        InitialLightingRelaxation, WorldLoadingPhase, system_params::WorldSetupProgress,
    },
};

const INITIAL_LIGHTING_RELAXATION_BATCH: usize = 8;

pub(super) fn light_initial_chunks(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    lighting: &mut PendingLightingUpdates,
    changed_chunks: &mut HashSet<IVec3>,
) {
    let mut budget = FrameWorkBudget::new(INITIAL_LOADING_BUDGET, 1);

    seed_initial_direct_lighting(content, progress, lighting, &mut budget);
    if progress.loading_state.lighting_seed_cursor < progress.loading_state.coords.len() {
        return;
    }

    settle_initial_lighting(
        content,
        progress,
        lighting,
        changed_chunks,
        &mut budget,
    );
}

fn seed_initial_direct_lighting(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    lighting: &mut PendingLightingUpdates,
    budget: &mut FrameWorkBudget,
) {
    debug_assert!(
        lighting.is_empty(),
        "bootstrap direct-light seeding must run before relaxation is queued"
    );

    while progress.loading_state.lighting_seed_cursor < progress.loading_state.coords.len() {
        if budget.exhausted() {
            break;
        }

        let coord = progress.loading_state.coords[progress.loading_state.lighting_seed_cursor];
        let chunk_is_empty = progress
            .world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated bootstrap chunk data should exist at {coord:?}"))
            .is_empty();
        let direct_seed = lighting.seed_chunk_direct_lighting(
            &mut progress.world,
            coord,
            content.blocks(),
            content.fluids(),
            content.secondary_properties(),
        );

        if chunk_is_empty {
            progress
                .loading_state
                .lighting_relaxations
                .push(InitialLightingRelaxation::BoundaryOnly(coord));
        } else if direct_seed.requires_relaxation {
            progress
                .loading_state
                .lighting_relaxations
                .push(InitialLightingRelaxation::Full(coord));
        }

        progress.loading_state.lighting_seed_cursor += 1;
        budget.record(1);

        let total = progress.loading_state.coords.len();
        progress.loading_state.lit = progress
            .loading_state
            .lighting_seed_cursor
            .min(total.saturating_sub(1));
    }
}

fn settle_initial_lighting(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    lighting: &mut PendingLightingUpdates,
    changed_chunks: &mut HashSet<IVec3>,
    budget: &mut FrameWorkBudget,
) {
    let mut changed_positions = HashSet::new();

    loop {
        if budget.exhausted() {
            break;
        }

        if lighting.is_empty() {
            enqueue_initial_relaxation_batch(progress, lighting);
            if lighting.is_empty() {
                finish_initial_lighting(progress);
                break;
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
    }

    if progress.loading_state.lighting_relaxation_cursor
        >= progress.loading_state.lighting_relaxations.len()
        && lighting.is_empty()
    {
        finish_initial_lighting(progress);
    }
}

fn enqueue_initial_relaxation_batch(
    progress: &mut WorldSetupProgress<'_>,
    lighting: &mut PendingLightingUpdates,
) {
    let start = progress.loading_state.lighting_relaxation_cursor;
    let end = start
        .saturating_add(INITIAL_LIGHTING_RELAXATION_BATCH)
        .min(progress.loading_state.lighting_relaxations.len());

    for relaxation in progress.loading_state.lighting_relaxations[start..end]
        .iter()
        .copied()
    {
        match relaxation {
            InitialLightingRelaxation::BoundaryOnly(coord) => {
                lighting.enqueue_empty_chunk_relaxation(coord);
            }
            InitialLightingRelaxation::Full(coord) => {
                lighting.enqueue_chunk_relaxation(coord);
            }
        }
    }

    progress.loading_state.lighting_relaxation_cursor = end;
}

fn finish_initial_lighting(progress: &mut WorldSetupProgress<'_>) {
    progress.loading_state.lit = progress.loading_state.coords.len();
    progress.loading_state.lighting_relaxations.clear();
    progress.loading_state.phase = WorldLoadingPhase::Meshing;
}
