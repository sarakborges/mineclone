use std::time::Duration;

use crate::app::crash_log::log_gameplay_event;
use crate::world::{
    PendingFluidUpdates,
    chunk_system_params::ChunkContent,
    work_budget::FrameWorkBudget,
};

use super::super::{WorldLoadingPhase, system_params::WorldSetupProgress};

const INITIAL_FLUID_SETTLING_BUDGET: Duration = Duration::from_millis(4);
const MIN_INITIAL_FLUID_SETTLING_UPDATES: usize = 16;
const MAX_INITIAL_FLUID_SETTLING_UPDATES: usize = 1_024;
pub(super) fn settle_initial_fluids(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
) {
    if !progress.loading_state.fluid_settling.is_active() {
        let coords = progress.loading_state.coords.clone();
        log_gameplay_event(format!("world.loading.fluid_settling.start generated_chunks={}", coords.len()));
        progress
            .loading_state
            .fluid_settling
            .begin(&progress.world, content.fluids(), coords);
    }

    let mut budget = FrameWorkBudget::new(
        INITIAL_FLUID_SETTLING_BUDGET,
        MIN_INITIAL_FLUID_SETTLING_UPDATES,
    )
    .with_maximum_items(MAX_INITIAL_FLUID_SETTLING_UPDATES);

    let complete = {
        let progress = &mut *progress;
        progress.loading_state.fluid_settling.process(
            &mut progress.world,
            content.fluids(),
            &mut budget,
        )
    };

    if complete {
        let completion = progress
            .loading_state
            .fluid_settling
            .take_completion()
            .expect("completed initial fluid settling must own its generated chunk set");
        debug_assert_eq!(
            completion.generated_chunks.len(),
            progress.loading_state.coords.len()
        );
        debug_assert!(
            completion.changed_existing_positions.is_empty(),
            "bootstrap settling should not need a previously published halo"
        );

        // Runtime begins from the verified converged generated state. Frontier
        // work beyond the resident bootstrap closure remains runtime-owned.
        *fluid_updates = PendingFluidUpdates::default();
        for coord in completion.generated_chunks {
            fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, coord);
        }

        log_gameplay_event(format!("world.loading.fluid_settling.complete generated_chunks={} changed_existing_positions={} owned_existing_chunks={}", progress.loading_state.coords.len(), completion.changed_existing_positions.len(), completion.owned_existing_chunks.len()));
        progress.loading_state.phase = WorldLoadingPhase::Lighting;
    }
}
