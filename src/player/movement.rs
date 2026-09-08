use bevy::prelude::*;

use crate::app::{game_state::GameState, pause_state::PauseState};
use flight::{handle_flight_toggle, move_flying};
use gravity::apply_gravity;
use walking::walk;
use world_bounds::enforce_world_floor;

mod collision;
mod config;
pub(crate) mod flight;
pub(crate) mod gravity;
mod smoothing;
pub(crate) mod walking;
mod world_bounds;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_flight_toggle,
                walk,
                move_flying,
                apply_gravity,
                enforce_world_floor,
            )
                .chain()
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(PauseState::Running)),
        );
    }
}
