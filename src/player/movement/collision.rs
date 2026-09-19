use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT},
    voxel::{
        collision::{collides_aabb, try_step_up_aabb},
        world::VoxelWorld,
    },
};

use super::config::COLLISION_STEP;

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
    if delta == 0.0 {
        return MoveAxisResult::Clear;
    }

    let steps = (delta.abs() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = delta / steps as f32;
    let mut moved = 0.0;

    for _ in 0..steps {
        translate_axis(&mut transform.translation, step, axis);

        if player_collides(transform.translation, world) {
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
                    player_bounds,
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

pub(super) fn player_collides(eye_position: Vec3, world: &VoxelWorld) -> bool {
    let (min, max) = player_bounds(eye_position);
    collides_aabb(world, min, max)
}

fn player_bounds(eye_position: Vec3) -> (Vec3, Vec3) {
    let feet_y = eye_position.y - PLAYER_EYE_HEIGHT;
    let min = Vec3::new(
        eye_position.x - PLAYER_HALF_WIDTH,
        feet_y,
        eye_position.z - PLAYER_HALF_WIDTH,
    );
    let max = Vec3::new(
        eye_position.x + PLAYER_HALF_WIDTH,
        feet_y + PLAYER_HEIGHT,
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
