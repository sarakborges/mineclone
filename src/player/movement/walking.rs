use bevy::prelude::*;

use crate::{
    player::{PlayerEntity, camera::GameplayCamera},
    voxel::{collision::ENTITY_STEP_HEIGHT, world::VoxelWorld},
    world::{game_rules::GameRules, tick::WorldTickClock},
};

use super::{
    collision::{Axis, MoveAxisResult, move_axis},
    config::{
        DOUBLE_TAP_WINDOW_TICKS, RUN_SPEED_MULTIPLIER, STEP_SMOOTH_SPEED, WALK_ACCELERATION,
        WALK_DECELERATION, WALK_SPEED,
    },
    flight::FlightState,
    gravity::GravityState,
    smoothing::approach_velocity,
};

#[derive(Component, Default)]
pub struct WalkingState {
    velocity: Vec3,
    step_target_y: Option<f32>,
    running: bool,
    run_deadline_tick: Option<u64>,
}

impl WalkingState {
    pub(crate) fn horizontal_speed_squared(&self) -> f32 {
        self.velocity.x * self.velocity.x + self.velocity.z * self.velocity.z
    }

    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    pub(crate) fn reset_motion(&mut self) {
        self.velocity = Vec3::ZERO;
        self.step_target_y = None;
        self.running = false;
        self.run_deadline_tick = None;
    }
}

pub(super) fn walk(
    game_rules: Res<GameRules>,
    world_ticks: Res<WorldTickClock>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<VoxelWorld>,
    camera: Single<&GameplayCamera>,
    player: Single<
        (&mut Transform, &FlightState, &mut GravityState, &mut WalkingState),
        With<PlayerEntity>,
    >,
) {
    let camera = camera.into_inner();
    let (mut transform, flight, mut gravity, mut walking) = player.into_inner();

    if flight.active {
        if walking.velocity != Vec3::ZERO {
            walking.velocity = Vec3::ZERO;
        }
        walking.running = false;
        walking.run_deadline_tick = None;
        return;
    }

    update_run_state(&keys, &world_ticks, &mut walking);

    let delta_seconds = world_ticks.delta_seconds(&game_rules);
    if delta_seconds <= 0.0 {
        return;
    }

    if let Some(target_y) = walking.step_target_y {
        gravity.grounded = true;
        gravity.vertical_velocity = 0.0;
        transform.translation.y = approach_step_height(
            transform.translation.y,
            target_y,
            STEP_SMOOTH_SPEED * delta_seconds,
        );
        if (transform.translation.y - target_y).abs() <= f32::EPSILON {
            transform.translation.y = target_y;
            walking.step_target_y = None;
        }
    }

    let yaw_rotation = Quat::from_rotation_y(camera.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let mut input = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        input += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        input -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        input += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        input -= right;
    }

    let move_speed = if walking.running {
        WALK_SPEED * RUN_SPEED_MULTIPLIER
    } else {
        WALK_SPEED
    };
    let target_velocity = if input.length_squared() > 0.0 {
        input.normalize() * move_speed
    } else {
        Vec3::ZERO
    };
    let acceleration = if target_velocity == Vec3::ZERO {
        WALK_DECELERATION
    } else {
        WALK_ACCELERATION
    };
    let next_velocity = approach_velocity(
        walking.velocity,
        target_velocity,
        acceleration * delta_seconds,
    );

    if walking.velocity != next_velocity {
        walking.velocity = next_velocity;
    }

    let step_up_height = gravity.grounded.then_some(ENTITY_STEP_HEIGHT);
    let velocity = walking.velocity;
    if velocity.x != 0.0
        && !move_horizontal_axis(
            &mut transform,
            &world,
            velocity.x * delta_seconds,
            Axis::X,
            step_up_height,
            &mut walking.step_target_y,
        )
    {
        walking.velocity.x = 0.0;
    }
    if velocity.z != 0.0
        && !move_horizontal_axis(
            &mut transform,
            &world,
            velocity.z * delta_seconds,
            Axis::Z,
            step_up_height,
            &mut walking.step_target_y,
        )
    {
        walking.velocity.z = 0.0;
    }
}

fn update_run_state(
    keys: &ButtonInput<KeyCode>,
    world_ticks: &WorldTickClock,
    walking: &mut WalkingState,
) {
    if !keys.pressed(KeyCode::KeyW) {
        walking.running = false;
    }

    let current_tick = world_ticks.current_tick();
    if walking
        .run_deadline_tick
        .is_some_and(|deadline| current_tick > deadline)
    {
        walking.run_deadline_tick = None;
    }

    if !keys.just_pressed(KeyCode::KeyW) {
        return;
    }

    if walking
        .run_deadline_tick
        .is_some_and(|deadline| current_tick <= deadline)
    {
        walking.running = true;
        walking.run_deadline_tick = None;
    } else {
        walking.run_deadline_tick =
            Some(current_tick.saturating_add(DOUBLE_TAP_WINDOW_TICKS));
    }
}

fn move_horizontal_axis(
    transform: &mut Transform,
    world: &VoxelWorld,
    delta: f32,
    axis: Axis,
    step_up_height: Option<f32>,
    step_target_y: &mut Option<f32>,
) -> bool {
    let visual_y = transform.translation.y;
    if let Some(target_y) = *step_target_y {
        transform.translation.y = target_y;
    }

    let result = move_axis(transform, world, delta, axis, step_up_height);
    match result {
        MoveAxisResult::Clear => {
            transform.translation.y = visual_y;
            true
        }
        MoveAxisResult::Blocked => {
            transform.translation.y = visual_y;
            false
        }
        MoveAxisResult::Stepped(position) => {
            *step_target_y = Some(position.y);
            transform.translation.y = visual_y;
            true
        }
    }
}

fn approach_step_height(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    if delta.abs() <= max_delta || max_delta <= 0.0 {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

#[cfg(test)]
mod tests {
    use super::approach_step_height;

    #[test]
    fn step_height_moves_toward_target_without_overshoot() {
        assert_eq!(approach_step_height(1.0, 2.0, 0.25), 1.25);
        assert_eq!(approach_step_height(1.9, 2.0, 0.25), 2.0);
        assert_eq!(approach_step_height(2.0, 1.0, 0.25), 1.75);
    }
}
