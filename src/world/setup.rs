mod bootstrap;
mod progress;
mod system_params;

use bevy::prelude::*;

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
    Spawning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldLoadingStage {
    Terrain,
    Chunks,
    Assets,
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
            WorldLoadingPhase::Assets | WorldLoadingPhase::Spawning => WorldLoadingStage::Assets,
        }
    }

    pub(crate) fn stage_progress(&self) -> (usize, usize) {
        match self.stage() {
            WorldLoadingStage::Terrain => (self.generated, self.total()),
            WorldLoadingStage::Chunks => (self.meshed, self.total()),
            WorldLoadingStage::Assets => (self.assets_loaded, self.assets_total),
        }
    }
}
