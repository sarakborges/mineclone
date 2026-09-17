use bevy::prelude::*;

use crate::{
    player::{PlayerEntity, camera::GameplayCamera},
    voxel::{collision::ENTITY_STEP_HEIGHT, world::VoxelWorld},
    world::{game_rules::GameRules, tick::WorldTickClock},
};

use super::{
    collision::{Axis, move_axis},
    config::{WALK_ACCELERATION, WALK_DECELERATION, WALK_SPEED},
    flight::FlightState,
    gravity::GravityState,
    smoothing::approach_velocity,
};

#[derive(Component, Default)]
pub struct WalkingState {
    velocity: Vec3,
}

pub(super) fn walk(
    game_rules: Res<GameRules>,
    world_ticks: Res<WorldTickClock>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    camera: Single<&GameplayCamera>,
    player: Single<
        (&mut Transform, &FlightState, &GravityState, &mut WalkingState),
        With<PlayerEntity>,
    >,
) {
    let camera = camera.into_inner();
    let (mut transform, flight, gravity, mut walking) = player.into_inner();

    if flight.active {
        if walking.velocity != Vec3::ZERO {
            walking.velocity = Vec3::ZERO;
        }
        return;
    }

    let delta_seconds = world_ticks.delta_seconds(&game_rules);
    if delta_seconds <= 0.0 {
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
    let next_velocity = approach_velocity(
        walking.velocity,
        target_velocity,
        acceleration * delta_seconds,
    );

    if walking.velocity != next_velocity {
        walking.velocity = next_velocity;
    }

    let step_up_height = gravity.grounded.then_some(ENTITY_STEP_HEIGHT);
    let velocity = walking.velocity;
    if velocity.x != 0.0
        && move_axis(
            &mut transform,
            &world,
            velocity.x * delta_seconds,
            Axis::X,
            step_up_height,
        )
    {
        walking.velocity.x = 0.0;
    }
    if velocity.z != 0.0
        && move_axis(
            &mut transform,
            &world,
            velocity.z * delta_seconds,
            Axis::Z,
            step_up_height,
        )
    {
        walking.velocity.z = 0.0;
    }
}
