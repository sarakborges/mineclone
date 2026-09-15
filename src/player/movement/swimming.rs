use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, PLAYER_HEIGHT, camera::GameplayCamera},
    voxel::world::VoxelWorld,
};

use super::{
    collision::{Axis, move_axis},
    config::{
        JUMP_SPEED, SWIM_ASCEND_SPEED, SWIM_BUOYANCY_SPEED, SWIM_DESCEND_SPEED,
        SWIM_EXIT_SURFACE_MARGIN, SWIM_VERTICAL_ACCELERATION,
    },
    flight::FlightState,
    gravity::GravityState,
    vertical::VerticalMovementContext,
};

#[derive(Component, Default)]
pub struct SwimmingState {
    pub(super) active: bool,
}

pub(super) fn update_swimming_state(
    world: Res<VoxelWorld>,
    player: Single<(&Transform, &mut SwimmingState), With<GameplayCamera>>,
) {
    let (transform, mut swimming) = player.into_inner();
    let active = player_in_fluid(transform.translation, &world);
    if swimming.active != active {
        swimming.active = active;
    }
}

pub(super) fn swim_vertical(
    context: VerticalMovementContext,
    mut transform: Single<&mut Transform, With<GameplayCamera>>,
    flight: Single<&FlightState>,
    swimming: Single<&SwimmingState>,
    mut gravity: Single<&mut GravityState>,
) {
    if flight.active || !swimming.active {
        return;
    }

    if gravity.grounded {
        gravity.grounded = false;
    }

    let target_velocity = if context.keys.pressed(KeyCode::Space) {
        if player_near_fluid_surface(transform.translation, &context.world) {
            JUMP_SPEED
        } else {
            SWIM_ASCEND_SPEED
        }
    } else if context.keys.pressed(KeyCode::ShiftLeft)
        || context.keys.pressed(KeyCode::ShiftRight)
    {
        -SWIM_DESCEND_SPEED
    } else {
        SWIM_BUOYANCY_SPEED
    };

    let delta_seconds = context.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    let next_velocity = approach(
        gravity.vertical_velocity,
        target_velocity,
        SWIM_VERTICAL_ACCELERATION * delta_seconds,
    );
    if gravity.vertical_velocity != next_velocity {
        gravity.vertical_velocity = next_velocity;
    }

    let vertical_delta = gravity.vertical_velocity * delta_seconds;
    if vertical_delta != 0.0
        && move_axis(
            &mut transform,
            &context.world,
            vertical_delta,
            Axis::Y,
        )
    {
        gravity.vertical_velocity = 0.0;
    }
}

pub(super) fn player_in_fluid(eye_position: Vec3, world: &VoxelWorld) -> bool {
    player_fluid_surface(eye_position, world)
        .is_some_and(|surface_y| player_fluid_sample_y(eye_position) < surface_y)
}

fn player_near_fluid_surface(eye_position: Vec3, world: &VoxelWorld) -> bool {
    let sample_y = player_fluid_sample_y(eye_position);

    player_fluid_surface(eye_position, world).is_some_and(|surface_y| {
        let depth = surface_y - sample_y;
        depth > 0.0 && depth <= SWIM_EXIT_SURFACE_MARGIN
    })
}

fn player_fluid_surface(eye_position: Vec3, world: &VoxelWorld) -> Option<f32> {
    let sample_y = player_fluid_sample_y(eye_position);
    let voxel = Vec3::new(eye_position.x, sample_y, eye_position.z)
        .floor()
        .as_ivec3();
    let fluid = world.fluid_at(voxel)?;

    Some(voxel.y as f32 + fluid.height())
}

fn player_fluid_sample_y(eye_position: Vec3) -> f32 {
    let feet_y = eye_position.y - PLAYER_EYE_HEIGHT;
    feet_y + PLAYER_HEIGHT * 0.5
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;

    if delta.abs() <= max_delta {
        target
    } else {
        current + delta.signum() * max_delta
    }
}
