use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, voxel::world::VoxelWorld};

use super::{
    collision::{Axis, move_axis},
    config::{
        FLIGHT_TOGGLE_WINDOW_SECONDS, FLY_ACCELERATION, FLY_DECELERATION, FLY_SPEED_MULTIPLIER,
        WALK_SPEED,
    },
    gravity::GravityState,
    smoothing::approach_velocity,
    swimming::SwimmingState,
};

#[derive(Component)]
pub struct FlightState {
    pub(super) active: bool,
    toggle_window: f32,
    velocity: Vec3,
}

impl Default for FlightState {
    fn default() -> Self {
        Self {
            active: false,
            toggle_window: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

pub(super) fn handle_flight_toggle(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    swimming: Single<&SwimmingState>,
    mut flight: Single<&mut FlightState>,
    mut gravity: Single<&mut GravityState>,
) {
    flight.toggle_window = (flight.toggle_window - time.delta_secs()).max(0.0);

    if swimming.active && !flight.active {
        flight.toggle_window = 0.0;
        return;
    }

    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    if flight.toggle_window > 0.0 {
        flight.active = !flight.active;
        flight.toggle_window = 0.0;
        flight.velocity = Vec3::ZERO;
        gravity.vertical_velocity = 0.0;
        gravity.grounded = false;
    } else {
        flight.toggle_window = FLIGHT_TOGGLE_WINDOW_SECONDS;
    }
}

pub(super) fn move_flying(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    player: Single<(&mut Transform, &GameplayCamera, &mut FlightState)>,
) {
    let (mut transform, camera, mut flight) = player.into_inner();

    if !flight.active {
        flight.velocity = Vec3::ZERO;
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
        acceleration * time.delta_secs(),
    );

    let velocity = flight.velocity;

    if move_axis(
        &mut transform,
        &world,
        velocity.x * time.delta_secs(),
        Axis::X,
    ) {
        flight.velocity.x = 0.0;
    }
    if move_axis(
        &mut transform,
        &world,
        velocity.z * time.delta_secs(),
        Axis::Z,
    ) {
        flight.velocity.z = 0.0;
    }
    if move_axis(
        &mut transform,
        &world,
        velocity.y * time.delta_secs(),
        Axis::Y,
    ) {
        flight.velocity.y = 0.0;
    }
}
