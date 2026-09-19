use bevy::prelude::*;

use crate::{
    player::PlayerEntity,
    voxel::world::VoxelWorld,
};

use super::{
    collision::{Axis, MoveAxisResult, move_axis, player_collides},
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
    mut transform: Single<&mut Transform, With<PlayerEntity>>,
    flight: Single<&FlightState>,
    swimming: Single<&SwimmingState>,
    mut gravity: Single<&mut GravityState>,
) {
    if flight.active || swimming.active {
        return;
    }

    if gravity.grounded {
        if !has_ground_support(&transform, &context.world) {
            gravity.grounded = false;
        } else if context.keys.just_pressed(KeyCode::Space) {
            gravity.vertical_velocity = JUMP_SPEED;
            gravity.grounded = false;
        } else {
            return;
        }
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
        None,
    );

    if matches!(hit_vertical_surface, MoveAxisResult::Blocked | MoveAxisResult::Stepped(_)) {
        if gravity.vertical_velocity < 0.0 {
            gravity.grounded = true;
        }

        gravity.vertical_velocity = 0.0;
    }
}

fn has_ground_support(transform: &Transform, world: &VoxelWorld) -> bool {
    player_collides(transform.translation - Vec3::Y * GROUND_PROBE, world)
}
