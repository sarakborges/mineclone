use bevy::prelude::*;

use crate::{
    player::{camera::GameplayCamera, game_mode::GameMode},
    voxel::world::VoxelWorld,
    world::{game_rules::GameRules, tick::WorldTickClock},
};

use super::{
    collision::{Axis, move_axis},
    config::{
        FLIGHT_TOGGLE_WINDOW_TICKS, FLY_ACCELERATION, FLY_DECELERATION, FLY_SPEED_MULTIPLIER,
        WALK_SPEED,
    },
    gravity::GravityState,
    smoothing::approach_velocity,
};

#[derive(Component)]
pub struct FlightState {
    pub(super) active: bool,
    toggle_deadline_tick: Option<u64>,
    velocity: Vec3,
}

impl Default for FlightState {
    fn default() -> Self {
        Self {
            active: false,
            toggle_deadline_tick: None,
            velocity: Vec3::ZERO,
        }
    }
}

pub(super) fn handle_flight_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    world_ticks: Res<WorldTickClock>,
    game_mode: Single<&GameMode>,
    mut flight: Single<&mut FlightState>,
    mut gravity: Single<&mut GravityState>,
) {
    if !game_mode.allows_flight() {
        flight.toggle_deadline_tick = None;

        if flight.active {
            flight.active = false;
            flight.velocity = Vec3::ZERO;
            gravity.vertical_velocity = 0.0;
            gravity.grounded = false;
        }

        return;
    }

    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    let current_tick = world_ticks.current_tick();
    if flight
        .toggle_deadline_tick
        .is_some_and(|deadline| current_tick <= deadline)
    {
        flight.active = !flight.active;
        flight.toggle_deadline_tick = None;
        flight.velocity = Vec3::ZERO;
        gravity.vertical_velocity = 0.0;
        gravity.grounded = false;
    } else {
        flight.toggle_deadline_tick = Some(current_tick.saturating_add(FLIGHT_TOGGLE_WINDOW_TICKS));
    }
}

pub(super) fn move_flying(
    game_rules: Res<GameRules>,
    world_ticks: Res<WorldTickClock>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    player: Single<(&mut Transform, &GameplayCamera, &mut FlightState)>,
) {
    let (mut transform, camera, mut flight) = player.into_inner();

    if !flight.active {
        flight.velocity = Vec3::ZERO;
        return;
    }

    let delta_seconds = world_ticks.delta_seconds(&game_rules);
    if delta_seconds <= 0.0 {
        return;
    }

    let fly_speed = WALK_SPEED * FLY_SPEED_MULTIPLIER;
    let yaw_rotation = Quat::from_rotation_y(camera.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let mut horizontal_input = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        horizontal_input += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        horizontal_input -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        horizontal_input += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        horizontal_input -= right;
    }

    let mut target_velocity = if horizontal_input.length_squared() > 0.0 {
        horizontal_input.normalize() * fly_speed
    } else {
        Vec3::ZERO
    };

    if keys.pressed(KeyCode::Space) {
        target_velocity.y += fly_speed;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        target_velocity.y -= fly_speed;
    }

    let acceleration = if target_velocity == Vec3::ZERO {
        FLY_DECELERATION
    } else {
        FLY_ACCELERATION
    };

    flight.velocity = approach_velocity(
        flight.velocity,
        target_velocity,
        acceleration * delta_seconds,
    );

    let velocity = flight.velocity;

    if move_axis(
        &mut transform,
        &world,
        velocity.x * delta_seconds,
        Axis::X,
    ) {
        flight.velocity.x = 0.0;
    }
    if move_axis(
        &mut transform,
        &world,
        velocity.z * delta_seconds,
        Axis::Z,
    ) {
        flight.velocity.z = 0.0;
    }
    if move_axis(
        &mut transform,
        &world,
        velocity.y * delta_seconds,
        Axis::Y,
    ) {
        flight.velocity.y = 0.0;
    }
}
