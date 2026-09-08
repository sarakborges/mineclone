use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT},
    voxel::{collision::collides_aabb, world::VoxelWorld},
};

use super::config::COLLISION_STEP;

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
) -> bool {
    if delta == 0.0 {
        return false;
    }

    let steps = (delta.abs() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = delta / steps as f32;

    for _ in 0..steps {
        translate_axis(&mut transform.translation, step, axis);

        if player_collides(transform.translation, world) {
            translate_axis(&mut transform.translation, -step, axis);
            return true;
        }
    }

    false
}

pub(super) fn player_collides(eye_position: Vec3, world: &VoxelWorld) -> bool {
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

    collides_aabb(world, min, max)
}

fn translate_axis(position: &mut Vec3, amount: f32, axis: Axis) {
    match axis {
        Axis::X => position.x += amount,
        Axis::Y => position.y += amount,
        Axis::Z => position.z += amount,
    }
}
