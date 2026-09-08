use bevy::prelude::*;

use crate::{
    game_state::GameState,
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT},
    player_camera::GameplayCamera,
    voxel_chunk::VoxelChunk,
    voxel_collision::collides_aabb,
};

const WALK_SPEED: f32 = 5.0;
const GRAVITY: f32 = -18.0;
const JUMP_SPEED: f32 = 7.0;
const GROUND_PROBE: f32 = 0.05;
const COLLISION_STEP: f32 = 0.05;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_player.run_if(in_state(GameState::Gameplay)));
    }
}

#[derive(Component)]
pub struct PlayerMovement {
    vertical_velocity: f32,
    grounded: bool,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            vertical_velocity: 0.0,
            grounded: true,
        }
    }
}

fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    chunk: Single<&VoxelChunk>,
    mut player: Single<(&mut Transform, &GameplayCamera, &mut PlayerMovement)>,
) {
    let mut movement = Vec3::ZERO;
    let yaw_rotation = Quat::from_rotation_y(player.1.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;

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

    if movement.length_squared() > 0.0 {
        let horizontal = movement.normalize() * WALK_SPEED * time.delta_secs();
        move_axis(&mut player.0, &chunk, horizontal.x, Axis::X);
        move_axis(&mut player.0, &chunk, horizontal.z, Axis::Z);
    }

    if player.2.grounded && !has_ground_support(&player.0, &chunk) {
        player.2.grounded = false;
    }

    if keys.just_pressed(KeyCode::Space) && player.2.grounded {
        player.2.vertical_velocity = JUMP_SPEED;
        player.2.grounded = false;
    }

    player.2.vertical_velocity += GRAVITY * time.delta_secs();
    let vertical_delta = player.2.vertical_velocity * time.delta_secs();
    let hit_vertical_surface = move_axis(&mut player.0, &chunk, vertical_delta, Axis::Y);

    if hit_vertical_surface {
        if player.2.vertical_velocity < 0.0 {
            player.2.grounded = true;
        }

        player.2.vertical_velocity = 0.0;
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

fn move_axis(transform: &mut Transform, chunk: &VoxelChunk, delta: f32, axis: Axis) -> bool {
    if delta == 0.0 {
        return false;
    }

    let steps = (delta.abs() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = delta / steps as f32;

    for _ in 0..steps {
        translate_axis(&mut transform.translation, step, axis);

        if player_collides(transform.translation, chunk) {
            translate_axis(&mut transform.translation, -step, axis);
            return true;
        }
    }

    false
}

fn translate_axis(position: &mut Vec3, amount: f32, axis: Axis) {
    match axis {
        Axis::X => position.x += amount,
        Axis::Y => position.y += amount,
        Axis::Z => position.z += amount,
    }
}

fn has_ground_support(transform: &Transform, chunk: &VoxelChunk) -> bool {
    player_collides(transform.translation - Vec3::Y * GROUND_PROBE, chunk)
}

fn player_collides(eye_position: Vec3, chunk: &VoxelChunk) -> bool {
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

    collides_aabb(chunk, min, max)
}
