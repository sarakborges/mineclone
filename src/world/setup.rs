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
    Spawning,
}

#[derive(Resource)]
pub(crate) struct WorldLoadingState {
    coords: Vec<IVec3>,
    generation_cursor: usize,
    generated: usize,
    lit: usize,
    mesh_cursor: usize,
    meshed: usize,
    spawn_column: IVec2,
    fluid_settling: GeneratedFluidSettling,
    phase: WorldLoadingPhase,
    screen_rendered: bool,
    transition_requested: bool,
}

impl WorldLoadingState {
    pub(crate) fn generated(&self) -> usize {
        self.generated
    }

    pub(crate) fn total(&self) -> usize {
        self.coords.len()
    }
}
