mod bootstrap;
mod progress;
mod system_params;

use bevy::{platform::collections::HashMap, prelude::*};

use super::fluid_updates::GeneratedFluidSettling;

pub(super) use bootstrap::begin_world_loading;
pub(super) use progress::setup_world;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldLoadingPhase {
    Generating,
    SettlingFluids,
    Lighting,
    Meshing,
    Assets,
    Finalizing,
    Spawning,
}

impl WorldLoadingPhase {
    pub(crate) const ALL: [Self; 7] = [
        Self::Generating,
        Self::SettlingFluids,
        Self::Lighting,
        Self::Meshing,
        Self::Assets,
        Self::Finalizing,
        Self::Spawning,
    ];

    fn ordinal(self) -> u8 {
        match self {
            Self::Generating => 0,
            Self::SettlingFluids => 1,
            Self::Lighting => 2,
            Self::Meshing => 3,
            Self::Assets => 4,
            Self::Finalizing => 5,
            Self::Spawning => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldLoadingPhaseStatus {
    Pending,
    Active,
    Done,
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

    pub(crate) fn column_count(&self) -> usize {
        self.column_top_chunks.len()
    }

    pub(crate) fn phase_status(&self, phase: WorldLoadingPhase) -> WorldLoadingPhaseStatus {
        if phase == self.phase {
            if phase == WorldLoadingPhase::Spawning && self.transition_requested {
                WorldLoadingPhaseStatus::Done
            } else {
                WorldLoadingPhaseStatus::Active
            }
        } else if phase.ordinal() < self.phase.ordinal() {
            WorldLoadingPhaseStatus::Done
        } else {
            WorldLoadingPhaseStatus::Pending
        }
    }

    pub(crate) fn phase_progress(&self, phase: WorldLoadingPhase) -> Option<(usize, usize)> {
        match phase {
            WorldLoadingPhase::Generating => Some((self.generated, self.total())),
            WorldLoadingPhase::SettlingFluids => None,
            WorldLoadingPhase::Lighting => Some((self.lit, self.total())),
            WorldLoadingPhase::Meshing => Some((self.meshed, self.total())),
            WorldLoadingPhase::Assets => Some((self.assets_loaded, self.assets_total)),
            WorldLoadingPhase::Finalizing => Some((
                usize::from(self.finalization_frames),
                usize::from(progress::INITIAL_FINALIZATION_FRAMES),
            )),
            WorldLoadingPhase::Spawning => None,
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
