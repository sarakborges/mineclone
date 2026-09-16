use bevy::prelude::*;

use crate::gameplay::availability::world_interaction_available;
use flight::{handle_flight_toggle, move_flying};
use gravity::apply_gravity;
use swimming::{swim_vertical, update_swimming_state};
use walking::walk;
use world_bounds::enforce_world_floor;

mod collision;
pub(crate) mod config;
pub(crate) mod flight;
pub(crate) mod gravity;
mod smoothing;
pub(crate) mod swimming;
mod vertical;
pub(crate) mod walking;
mod world_bounds;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_swimming_state,
                handle_flight_toggle,
                walk,
                move_flying,
                swim_vertical,
                apply_gravity,
                enforce_world_floor,
            )
                .chain()
                .run_if(world_interaction_available),
        );
    }
}
