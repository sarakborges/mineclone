use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    voxel::world::VoxelWorld,
};

use super::{
    collision::{Axis, move_axis, player_collides},
    config::{GRAVITY, GROUND_PROBE, JUMP_SPEED},
    flight::FlightState,
    swimming::SwimmingState,
    vertical::VerticalMovementContext,
};

#[derive(Component)]
pub struct GravityState {
    pub(super) vertical_velocity: f32,
    pub(super) grounded: bool,
}

impl Default for GravityState {
    fn default() -> Self {
        Self {
            vertical_velocity: 0.0,
            grounded: true,
        }
    }
}

pub(super) fn apply_gravity(
    context: VerticalMovementContext,
    mut transform: Single<&mut Transform, With<GameplayCamera>>,
    flight: Single<&FlightState>,
    swimming: Single<&SwimmingState>,
    mut gravity: Single<&mut GravityState>,
) {
    if flight.active || swimming.active {
        return;
    }

    if gravity.grounded && !has_ground_support(&transform, &context.world) {
        gravity.grounded = false;
    }

    if context.keys.just_pressed(KeyCode::Space) && gravity.grounded {
        gravity.vertical_velocity = JUMP_SPEED;
        gravity.grounded = false;
    }

    let delta_seconds = context.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    gravity.vertical_velocity += GRAVITY * delta_seconds;
    let vertical_delta = gravity.vertical_velocity * delta_seconds;
    let hit_vertical_surface = move_axis(
        &mut transform,
        &context.world,
        vertical_delta,
        Axis::Y,
    );

    if hit_vertical_surface {
        if gravity.vertical_velocity < 0.0 {
            gravity.grounded = true;
        }

        gravity.vertical_velocity = 0.0;
    }
}

fn has_ground_support(transform: &Transform, world: &VoxelWorld) -> bool {
    player_collides(transform.translation - Vec3::Y * GROUND_PROBE, world)
}
