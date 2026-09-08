use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, voxel::world::VoxelWorld};

use super::{
    collision::{move_axis, Axis},
    config::{WALK_ACCELERATION, WALK_DECELERATION, WALK_SPEED},
    flight::FlightState,
    smoothing::approach_velocity,
};

#[derive(Component, Default)]
pub struct WalkingState {
    velocity: Vec3,
}

pub(super) fn walk(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    player: Single<(&mut Transform, &GameplayCamera, &FlightState, &mut WalkingState)>,
) {
    let (mut transform, camera, flight, mut walking) = player.into_inner();

    if flight.active {
        walking.velocity = Vec3::ZERO;
        return;
    }

    let yaw_rotation = Quat::from_rotation_y(camera.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let mut input = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        input += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        input -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        input += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        input -= right;
    }

    let target_velocity = if input.length_squared() > 0.0 {
        input.normalize() * WALK_SPEED
    } else {
        Vec3::ZERO
    };
    let acceleration = if target_velocity == Vec3::ZERO {
        WALK_DECELERATION
    } else {
        WALK_ACCELERATION
    };

    walking.velocity = approach_velocity(
        walking.velocity,
        target_velocity,
        acceleration * time.delta_secs(),
    );

    let velocity = walking.velocity;
    if move_axis(&mut transform, &world, velocity.x * time.delta_secs(), Axis::X) {
        walking.velocity.x = 0.0;
    }
    if move_axis(&mut transform, &world, velocity.z * time.delta_secs(), Axis::Z) {
        walking.velocity.z = 0.0;
    }
}
