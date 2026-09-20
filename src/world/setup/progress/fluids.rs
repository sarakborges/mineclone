use std::time::Duration;

use crate::world::{
    PendingFluidUpdates,
    chunk_system_params::ChunkContent,
    work_budget::FrameWorkBudget,
};

use super::super::{WorldLoadingPhase, system_params::WorldSetupProgress};

const INITIAL_FLUID_PRIMING_BUDGET: Duration = Duration::from_millis(4);
const MIN_INITIAL_FLUID_PRIMING_UPDATES: usize = 16;
const MAX_INITIAL_FLUID_PRIMING_UPDATES: usize = 1_024;

pub(super) fn prime_initial_fluids(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
    fluid_updates: &mut PendingFluidUpdates,
) {
    if !progress.loading_state.fluid_priming.is_active() {
        let coords = progress.loading_state.coords.clone();
        progress
            .loading_state
            .fluid_priming
            .begin(&progress.world, coords);
    }

    let mut budget = FrameWorkBudget::new(
        INITIAL_FLUID_PRIMING_BUDGET,
        MIN_INITIAL_FLUID_PRIMING_UPDATES,
    )
    .with_maximum_items(MAX_INITIAL_FLUID_PRIMING_UPDATES);

    let complete = {
        let progress = &mut *progress;
        progress.loading_state.fluid_priming.process(
            &mut progress.world,
            content.fluids(),
            &mut budget,
        )
    };

    if complete {
        let completed = progress
            .loading_state
            .fluid_priming
            .take_completed_chunks()
            .expect("completed initial fluid priming must own its generated chunk set");
        debug_assert_eq!(completed.len(), progress.loading_state.coords.len());

        // Generation-time frontier wakes describe the authored pre-prime
        // snapshot. Replace them with the actual post-prime frontier so the
        // first runtime scheduled tick necessarily continues from what the
        // player sees in the first mesh.
        *fluid_updates = PendingFluidUpdates::default();
        for coord in completed {
            fluid_updates.enqueue_loaded_fluid_frontier(&progress.world, coord);
        }

        progress.loading_state.phase = WorldLoadingPhase::Lighting;
    }
}
