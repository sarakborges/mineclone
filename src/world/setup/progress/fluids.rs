use bevy::platform::collections::HashSet;

use crate::world::{
    chunk_system_params::ChunkContent,
    fluid_updates::settle_generated_fluid_chunks,
};

use super::super::{WorldLoadingPhase, system_params::WorldSetupProgress};

pub(super) fn settle_initial_fluids(
    content: &ChunkContent<'_>,
    progress: &mut WorldSetupProgress<'_>,
) {
    let generated_chunks = progress
        .loading_state
        .coords
        .iter()
        .copied()
        .collect::<HashSet<_>>();

    settle_generated_fluid_chunks(
        &mut progress.world,
        content.fluids(),
        &generated_chunks,
    );
    progress.loading_state.phase = WorldLoadingPhase::Lighting;
}
