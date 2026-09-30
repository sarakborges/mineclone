use bevy::prelude::*;

use crate::{
    app::crash_log::log_gameplay_event,
    gameplay::availability::world_interaction_available,
    player::{PlayerEntity, game_mode::{GameMode, not_spectator}},
};
use flight::{handle_flight_toggle, move_flying};
use gravity::apply_gravity;
use swimming::{swim_vertical, update_swimming_state};
use walking::walk;
use world_bounds::enforce_world_floor;

pub(crate) mod collision;
pub(crate) mod config;
mod entity_collision;
pub(crate) mod flight;
pub(crate) mod gravity;
mod smoothing;
pub(crate) mod swimming;
mod vertical;
pub(crate) mod walking;
mod world_bounds;

#[derive(Default)]
struct MovementLogState {
    timer: Option<Timer>,
    last_position: Option<Vec3>,
    last_flying: Option<bool>,
    last_swimming: Option<bool>,
    last_grounded: Option<bool>,
    last_running: Option<bool>,
}

#[derive(SystemParam)]
struct MovementLogPlayer<'w, 's> {
    player: Single<'w, 's, (
        &'static Transform,
        &'static GameMode,
        &'static flight::FlightState,
        &'static swimming::SwimmingState,
        &'static gravity::GravityState,
        &'static walking::WalkingState,
    ), With<PlayerEntity>>,
}
fn log_movement_diagnostics(
    time: Res<Time<Real>>,
    player: MovementLogPlayer,
    mut state: Local<MovementLogState>,
) {
    let timer_finished = {
        let timer = state
            .timer
            .get_or_insert_with(|| Timer::from_seconds(1.0, TimerMode::Repeating));
        timer.tick(time.delta());
        timer.just_finished()
    };

    let (transform, game_mode, flight, swimming, gravity, walking) = player.player.into_inner();
    let flying = flight.is_active();
    let swimming = swimming.is_active();
    let grounded = gravity.grounded();
    let running = walking.is_running();
    let state_changed = state.last_flying != Some(flying)
        || state.last_swimming != Some(swimming)
        || state.last_grounded != Some(grounded)
        || state.last_running != Some(running);
    let moved = state.last_position.is_some_and(|previous| {
        previous.distance_squared(transform.translation) > 0.0001
    });

    if state_changed || (timer_finished && moved) {
        log_gameplay_event(format!(
            "player.movement position=({:.3},{:.3},{:.3}) mode={game_mode:?} flying={flying} swimming={swimming} grounded={grounded} running={running} horizontal_speed={:.3} vertical_velocity={:.3}",
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            walking.horizontal_speed_squared().sqrt(),
            gravity.vertical_velocity(),
        ));
    }

    state.last_position = Some(transform.translation);
    state.last_flying = Some(flying);
    state.last_swimming = Some(swimming);
    state.last_grounded = Some(grounded);
    state.last_running = Some(running);
}
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
                enforce_world_floor.run_if(not_spectator),
                log_movement_diagnostics,
            )
                .chain()
                .run_if(world_interaction_available),
        )
        .add_systems(
            PostUpdate,
            (
                entity_collision::resolve_creature_creature_contacts,
                entity_collision::resolve_player_creature_contacts.run_if(not_spectator),
            )
                .chain()
                .run_if(entity_collision::contacts_enabled()),
        );
    }
}
