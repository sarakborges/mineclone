mod bootstrap;
mod progress;
mod system_params;

use bevy::{platform::collections::HashMap, prelude::*};

use super::fluid_updates::GeneratedFluidSettling;

pub(super) use bootstrap::begin_world_loading;
pub(super) use progress::setup_world;

#[derive(Clone, Copy, Eq, PartialEq)]
enum WorldLoadingPhase {
    Generating,
    SettlingFluids,
    Lighting,
    Meshing,
    Assets,
    Finalizing,
    Spawning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldLoadingStage {
    Terrain,
    Chunks,
    Assets,
    Finalizing,
}

#[derive(Resource)]
pub(crate) struct WorldLoadingState {
    coords: Vec<IVec3>,
    generation_cursor: usize,
    generated: usize,
    lit: usize,
    mesh_cursor: usize,
    meshed: usize,
    assets_loaded: usize,
    assets_total: usize,
    finalization_frames: u8,
    column_top_chunks: HashMap<IVec2, i32>,
    spawn_column: IVec2,
    fluid_settling: GeneratedFluidSettling,
    phase: WorldLoadingPhase,
    screen_rendered: bool,
    transition_requested: bool,
}

impl WorldLoadingState {
    pub(crate) fn total(&self) -> usize {
        self.coords.len()
    }

    pub(crate) fn stage(&self) -> WorldLoadingStage {
        match self.phase {
            WorldLoadingPhase::Generating
            | WorldLoadingPhase::SettlingFluids
            | WorldLoadingPhase::Lighting => WorldLoadingStage::Terrain,
            WorldLoadingPhase::Meshing => WorldLoadingStage::Chunks,
            WorldLoadingPhase::Assets => WorldLoadingStage::Assets,
            WorldLoadingPhase::Finalizing | WorldLoadingPhase::Spawning => WorldLoadingStage::Finalizing,
        }
    }

    pub(crate) fn stage_progress(&self) -> (usize, usize) {
        match self.stage() {
            WorldLoadingStage::Terrain => (self.generated, self.total()),
            WorldLoadingStage::Chunks => (self.meshed, self.total()),
            WorldLoadingStage::Assets => (self.assets_loaded, self.assets_total),
            WorldLoadingStage::Finalizing => (
                usize::from(self.finalization_frames),
                usize::from(progress::INITIAL_FINALIZATION_FRAMES),
            ),
        }
    }

    fn extend_column_to_structure_top(&mut self, horizontal: IVec2, structure_top_chunk: i32) {
        let Some(current_top) = self.column_top_chunks.get_mut(&horizontal) else {
            return;
        };
        if structure_top_chunk <= *current_top {
            return;
        }

        let previous_top = *current_top;
        *current_top = structure_top_chunk;
        self.coords.extend(
            (previous_top + 1..=structure_top_chunk)
                .map(|y| IVec3::new(horizontal.x, y, horizontal.y)),
        );
    }
}
