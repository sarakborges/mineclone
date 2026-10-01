use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT},
    voxel::{
        collision::{collides_aabb, try_step_up_aabb},
        world::VoxelWorld,
    },
};

use super::config::{COLLISION_STEP, GROUND_PROBE};

#[derive(Clone, Copy)]
pub(super) enum MoveAxisResult {
    Clear,
    Blocked,
    Stepped(Vec3),
}

#[derive(Clone, Copy)]
pub(super) enum Axis {
    X,
    Y,
    Z,
}

pub(super) fn move_axis(
    transform: &mut Transform,
    world: &VoxelWorld,
    delta: f32,
    axis: Axis,
    step_up_height: Option<f32>,
) -> MoveAxisResult {
    move_axis_with_height(transform, world, delta, axis, step_up_height, PLAYER_HEIGHT)
}

pub(super) fn move_axis_with_height(
    transform: &mut Transform,
    world: &VoxelWorld,
    delta: f32,
    axis: Axis,
    step_up_height: Option<f32>,
    player_height: f32,
) -> MoveAxisResult {
    if delta == 0.0 {
        return MoveAxisResult::Clear;
    }

    let steps = (delta.abs() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = delta / steps as f32;
    let mut moved = 0.0;

    for _ in 0..steps {
        translate_axis(&mut transform.translation, step, axis);

        if player_collides_with_height(transform.translation, world, player_height) {
            translate_axis(&mut transform.translation, -step, axis);

            if let Some(max_step_height) = step_up_height {
                let remaining = delta - moved;
                let horizontal_delta = match axis {
                    Axis::X => Vec3::new(remaining, 0.0, 0.0),
                    Axis::Z => Vec3::new(0.0, 0.0, remaining),
                    Axis::Y => Vec3::ZERO,
                };
                if let Some(stepped_position) = try_step_up_aabb(
                    world,
                    transform.translation,
                    horizontal_delta,
                    max_step_height,
                    |position| player_bounds_for_height(position, player_height),
                ) {
                    transform.translation = stepped_position;
                    return MoveAxisResult::Stepped(stepped_position);
                }
            }

            return MoveAxisResult::Blocked;
        }
        moved += step;
    }

    MoveAxisResult::Clear
}

pub(crate) fn player_collides(eye_position: Vec3, world: &VoxelWorld) -> bool {
    player_collides_with_height(eye_position, world, PLAYER_HEIGHT)
}

pub(crate) fn player_collides_with_height(
    eye_position: Vec3,
    world: &VoxelWorld,
    player_height: f32,
) -> bool {
    let (min, max) = player_bounds_for_height(eye_position, player_height);
    collides_aabb(world, min, max)
}

pub(crate) fn player_has_ground_support(
    eye_position: Vec3,
    world: &VoxelWorld,
    player_height: f32,
) -> bool {
    let (min, max) = player_bounds_for_height(eye_position, player_height);
    let support_min = Vec3::new(min.x, min.y - GROUND_PROBE, min.z);
    let support_max = Vec3::new(max.x, min.y, max.z);
    collides_aabb(world, support_min, support_max)
}

pub(crate) fn player_bounds(eye_position: Vec3) -> (Vec3, Vec3) {
    player_bounds_for_height(eye_position, PLAYER_HEIGHT)
}

pub(crate) fn player_bounds_for_height(eye_position: Vec3, player_height: f32) -> (Vec3, Vec3) {
    let feet_y = eye_position.y - PLAYER_EYE_HEIGHT;
    let min = Vec3::new(
        eye_position.x - PLAYER_HALF_WIDTH,
        feet_y,
        eye_position.z - PLAYER_HALF_WIDTH,
    );
    let max = Vec3::new(
        eye_position.x + PLAYER_HALF_WIDTH,
        feet_y + player_height,
        eye_position.z + PLAYER_HALF_WIDTH,
    );
    (min, max)
}

fn translate_axis(position: &mut Vec3, amount: f32, axis: Axis) {
    match axis {
        Axis::X => position.x += amount,
        Axis::Y => position.y += amount,
        Axis::Z => position.z += amount,
    }
}
