use std::time::Duration;

use crate::world::{
    chunk_system_params::ChunkContent,
    work_budget::FrameWorkBudget,
};

use super::super::{WorldLoadingPhase, system_params::WorldSetupProgress};

const INITIAL_FLUID_SETTLING_BUDGET: Duration = Duration::from_millis(6);
const MIN_INITIAL_FLUID_SETTLING_UPDATES: usize = 32;
const MAX_INITIAL_FLUID_SETTLING_UPDATES: usize = 2_048;

pub(super) fn settle_initial_fluids(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
) {
    if !progress.loading_state.fluid_settling.is_active() {
        let coords = progress.loading_state.coords.clone();
        progress
            .loading_state
            .fluid_settling
            .begin(&progress.world, coords);
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
        let completed = progress
            .loading_state
            .fluid_settling
            .take_completed_chunks()
            .expect("completed initial fluid settling must own its generated chunk set");
        debug_assert_eq!(completed.len(), progress.loading_state.coords.len());
        progress.loading_state.phase = WorldLoadingPhase::Lighting;
    }
}
