use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, voxel::world::VoxelWorld};

use super::{
    collision::{move_axis, Axis},
    config::WALK_SPEED,
    flight::FlightState,
};

pub(super) fn walk(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    mut player: Single<(&mut Transform, &GameplayCamera, &FlightState)>,
) {
    if player.2.active {
        return;
    }

    let yaw_rotation = Quat::from_rotation_y(player.1.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let mut movement = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        movement += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        movement -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        movement += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        movement -= right;
    }

    if movement.length_squared() == 0.0 {
        return;
    }

    let horizontal = movement.normalize() * WALK_SPEED * time.delta_secs();
    move_axis(&mut player.0, &world, horizontal.x, Axis::X);
    move_axis(&mut player.0, &world, horizontal.z, Axis::Z);
}
