use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, voxel::world::VoxelWorld};

use super::{
    collision::{move_axis, player_collides, Axis},
    config::{GRAVITY, GROUND_PROBE, JUMP_SPEED},
    flight::FlightState,
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
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    mut transform: Single<&mut Transform, With<GameplayCamera>>,
    flight: Single<&FlightState>,
    mut gravity: Single<&mut GravityState>,
) {
    if flight.active {
        return;
    }

    if gravity.grounded && !has_ground_support(&transform, &world) {
        gravity.grounded = false;
    }

    if keys.just_pressed(KeyCode::Space) && gravity.grounded {
        gravity.vertical_velocity = JUMP_SPEED;
        gravity.grounded = false;
    }

    gravity.vertical_velocity += GRAVITY * time.delta_secs();
    let vertical_delta = gravity.vertical_velocity * time.delta_secs();
    let hit_vertical_surface = move_axis(&mut transform, &world, vertical_delta, Axis::Y);

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
