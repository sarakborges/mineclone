use bevy::prelude::*;

use crate::world::chunk_rendering::{ChunkRenderCoord, ChunkRenderPool};

use super::INITIAL_FINALIZATION_FRAMES;
use super::super::{WorldLoadingPhase, system_params::WorldSetupProgress};

pub(super) fn finalize_initial_world(
    render_pool: &ChunkRenderPool,
    progress: &mut WorldSetupProgress<'_>,
    chunk_entities: &Query<(), With<ChunkRenderCoord>>,
) {
    if render_pool.active_count() != progress.loading_state.total()
        || chunk_entities.iter().count() != render_pool.entity_count()
    {
        return;
    }

    progress.loading_state.finalization_frames = progress
        .loading_state
        .finalization_frames
        .saturating_add(1)
        .min(INITIAL_FINALIZATION_FRAMES);

    if progress.loading_state.finalization_frames >= INITIAL_FINALIZATION_FRAMES {
        progress.loading_state.phase = WorldLoadingPhase::Spawning;
    }
}
