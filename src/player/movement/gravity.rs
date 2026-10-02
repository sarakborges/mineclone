use bevy::prelude::*;

use crate::{
    app::keybinds::KeybindAction,
    player::PlayerEntity,
    world::current_context::CurrentDimensionContext,
};

use super::{
    collision::{Axis, MoveAxisResult, move_axis_with_height, player_has_ground_support},
    config::{FLY_SPEED, JUMP_SPEED},
    flight::FlightState,
    swimming::SwimmingState,
    vertical::VerticalMovementContext,
    walking::WalkingState,
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

impl GravityState {
    pub(crate) fn grounded(&self) -> bool {
        self.grounded
    }

    pub(crate) fn vertical_velocity(&self) -> f32 {
        self.vertical_velocity
    }

    pub(crate) fn reset_motion(&mut self) {
        self.vertical_velocity = 0.0;
        self.grounded = true;
    }
}

pub(super) fn apply_gravity(
    context: VerticalMovementContext,
    dimension: CurrentDimensionContext,
    player: Single<(&mut Transform, &WalkingState), With<PlayerEntity>>,
    flight: Single<&FlightState>,
    swimming: Single<&SwimmingState>,
    mut gravity: Single<&mut GravityState>,
) {
    if flight.active || swimming.active {
        return;
    }

    let Some(dimension) = dimension.definition() else {
        return;
    };
    let gravity_strength = dimension.gravity_strength;

    let (mut transform, walking) = player.into_inner();
    let player_height = walking.collision_height();

    if gravity.grounded {
        if !player_has_ground_support(transform.translation, &context.world, player_height) {
            gravity.grounded = false;
        } else if context.keys.just_pressed(context.keybinds.key_code(KeybindAction::Jump)) {
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

    gravity.vertical_velocity =
        (gravity.vertical_velocity - gravity_strength * delta_seconds).max(-FLY_SPEED);
    let vertical_delta = gravity.vertical_velocity * delta_seconds;
    let hit_vertical_surface = move_axis_with_height(
        &mut transform,
        &context.world,
        vertical_delta,
        Axis::Y,
        None,
        player_height,
    );

    if matches!(hit_vertical_surface, MoveAxisResult::Blocked | MoveAxisResult::Stepped(_)) {
        if gravity.vertical_velocity < 0.0 {
            gravity.grounded = true;
        }

        gravity.vertical_velocity = 0.0;
    }
}
