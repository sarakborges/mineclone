use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, voxel::world::VoxelWorld};

use super::{
    collision::{move_axis, Axis},
    config::{FLIGHT_TOGGLE_WINDOW_SECONDS, FLY_SPEED_MULTIPLIER, WALK_SPEED},
    gravity::GravityState,
};

#[derive(Component, Default)]
pub struct FlightState {
    pub(super) active: bool,
    toggle_window: f32,
}

pub(super) fn handle_flight_toggle(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut flight: Single<&mut FlightState>,
    mut gravity: Single<&mut GravityState>,
) {
    flight.toggle_window = (flight.toggle_window - time.delta_secs()).max(0.0);

    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    if flight.toggle_window > 0.0 {
        flight.active = !flight.active;
        flight.toggle_window = 0.0;
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
    mut player: Single<(&mut Transform, &GameplayCamera, &FlightState)>,
) {
    if !player.2.active {
        return;
    }

    let fly_speed = WALK_SPEED * FLY_SPEED_MULTIPLIER;
    let yaw_rotation = Quat::from_rotation_y(player.1.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let mut horizontal = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        horizontal += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        horizontal -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        horizontal += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        horizontal -= right;
    }

    if horizontal.length_squared() > 0.0 {
        let delta = horizontal.normalize() * fly_speed * time.delta_secs();
        move_axis(&mut player.0, &world, delta.x, Axis::X);
        move_axis(&mut player.0, &world, delta.z, Axis::Z);
    }

    let mut vertical_direction = 0.0;

    if keys.pressed(KeyCode::Space) {
        vertical_direction += 1.0;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        vertical_direction -= 1.0;
    }

    if vertical_direction != 0.0 {
        move_axis(
            &mut player.0,
            &world,
            vertical_direction * fly_speed * time.delta_secs(),
            Axis::Y,
        );
    }
}
