use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT},
    voxel::{collision::collides_aabb, world::VoxelWorld},
};

const WALK_SPEED: f32 = 5.0;
const FLY_SPEED_MULTIPLIER: f32 = 5.0;
const GRAVITY: f32 = -18.0;
const JUMP_SPEED: f32 = 7.0;
const FLIGHT_TOGGLE_WINDOW_SECONDS: f32 = 0.30;
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
    flying: bool,
    flight_toggle_window: f32,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            vertical_velocity: 0.0,
            grounded: true,
            flying: false,
            flight_toggle_window: 0.0,
        }
    }
}

fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    mut player: Single<(&mut Transform, &GameplayCamera, &mut PlayerMovement)>,
) {
    let delta_seconds = time.delta_secs();
    player.2.flight_toggle_window =
        (player.2.flight_toggle_window - delta_seconds).max(0.0);

    handle_flight_toggle(&keys, &mut player.2);

    let yaw_rotation = Quat::from_rotation_y(player.1.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let mut horizontal_movement = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        horizontal_movement += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        horizontal_movement -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        horizontal_movement += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        horizontal_movement -= right;
    }

    if horizontal_movement.length_squared() > 0.0 {
        let speed = if player.2.flying {
            WALK_SPEED * FLY_SPEED_MULTIPLIER
        } else {
            WALK_SPEED
        };
        let horizontal = horizontal_movement.normalize() * speed * delta_seconds;
        move_axis(&mut player.0, &world, horizontal.x, Axis::X);
        move_axis(&mut player.0, &world, horizontal.z, Axis::Z);
    }

    if player.2.flying {
        move_flying_vertical(&keys, delta_seconds, &world, &mut player.0, &mut player.2);
        return;
    }

    move_grounded_vertical(&keys, delta_seconds, &world, &mut player.0, &mut player.2);
}

fn handle_flight_toggle(keys: &ButtonInput<KeyCode>, movement: &mut PlayerMovement) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    if movement.flight_toggle_window > 0.0 {
        movement.flying = !movement.flying;
        movement.vertical_velocity = 0.0;
        movement.grounded = false;
        movement.flight_toggle_window = 0.0;
    } else {
        movement.flight_toggle_window = FLIGHT_TOGGLE_WINDOW_SECONDS;
    }
}

fn move_flying_vertical(
    keys: &ButtonInput<KeyCode>,
    delta_seconds: f32,
    world: &VoxelWorld,
    transform: &mut Transform,
    movement: &mut PlayerMovement,
) {
    movement.vertical_velocity = 0.0;
    movement.grounded = false;

    let mut vertical_direction = 0.0;

    if keys.pressed(KeyCode::Space) {
        vertical_direction += 1.0;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        vertical_direction -= 1.0;
    }

    if vertical_direction == 0.0 {
        return;
    }

    let fly_speed = WALK_SPEED * FLY_SPEED_MULTIPLIER;
    let vertical_delta = vertical_direction * fly_speed * delta_seconds;
    move_axis(transform, world, vertical_delta, Axis::Y);
}

fn move_grounded_vertical(
    keys: &ButtonInput<KeyCode>,
    delta_seconds: f32,
    world: &VoxelWorld,
    transform: &mut Transform,
    movement: &mut PlayerMovement,
) {
    if movement.grounded && !has_ground_support(transform, world) {
        movement.grounded = false;
    }

    if keys.just_pressed(KeyCode::Space) && movement.grounded {
        movement.vertical_velocity = JUMP_SPEED;
        movement.grounded = false;
    }

    movement.vertical_velocity += GRAVITY * delta_seconds;
    let vertical_delta = movement.vertical_velocity * delta_seconds;
    let hit_vertical_surface = move_axis(transform, world, vertical_delta, Axis::Y);

    if hit_vertical_surface {
        if movement.vertical_velocity < 0.0 {
            movement.grounded = true;
        }

        movement.vertical_velocity = 0.0;
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

fn move_axis(transform: &mut Transform, world: &VoxelWorld, delta: f32, axis: Axis) -> bool {
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

fn translate_axis(position: &mut Vec3, amount: f32, axis: Axis) {
    match axis {
        Axis::X => position.x += amount,
        Axis::Y => position.y += amount,
        Axis::Z => position.z += amount,
    }
}

fn has_ground_support(transform: &Transform, world: &VoxelWorld) -> bool {
    player_collides(transform.translation - Vec3::Y * GROUND_PROBE, world)
}

fn player_collides(eye_position: Vec3, world: &VoxelWorld) -> bool {
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
