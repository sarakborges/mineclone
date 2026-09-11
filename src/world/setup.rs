mod bootstrap;
mod progress;

use bevy::prelude::*;

pub(super) use bootstrap::begin_world_loading;
pub(super) use progress::setup_world;

#[derive(Clone, Copy, Eq, PartialEq)]
enum WorldLoadingPhase {
    Generating,
    Lighting,
    Meshing,
}

#[derive(Resource)]
pub(crate) struct WorldLoadingState {
    coords: Vec<IVec3>,
    generated: usize,
    lit: usize,
    meshed: usize,
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
