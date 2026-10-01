use bevy::prelude::*;

use crate::{
    app::keybinds::{KeybindAction, Keybinds},
    player::{PLAYER_HEIGHT, PlayerEntity, camera::{CameraPerspective, GameplayCamera}},
    voxel::{collision::ENTITY_STEP_HEIGHT, world::VoxelWorld},
    world::{game_rules::GameRules, tick::WorldTickClock},
};

use super::{
    collision::{
        Axis, MoveAxisResult, move_axis_with_height, player_collides_with_height,
        player_has_ground_support,
    },
    config::{
        COLLISION_STEP, DOUBLE_TAP_WINDOW_TICKS, RUN_SPEED_MULTIPLIER, STEP_SMOOTH_SPEED,
        WALK_ACCELERATION, WALK_DECELERATION, WALK_SPEED,
    },
    flight::FlightState,
    gravity::GravityState,
    smoothing::approach_velocity,
    swimming::SwimmingState,
};

pub(crate) const CROUCH_HEIGHT: f32 = 1.5;
pub(crate) const CROUCH_CAMERA_DROP: f32 = 0.35;
const CROUCH_SPEED_MULTIPLIER: f32 = 0.3;
const CROUCH_TRANSITION_SPEED: f32 = 8.0;

#[derive(Component, Default)]
pub struct WalkingState {
    velocity: Vec3,
    step_target_y: Option<f32>,
    running: bool,
    run_deadline_tick: Option<u64>,
    crouching: bool,
    crouch_blend: f32,
}

impl WalkingState {
    pub(crate) fn horizontal_speed_squared(&self) -> f32 {
        self.velocity.x * self.velocity.x + self.velocity.z * self.velocity.z
    }

    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    pub(crate) fn is_crouching(&self) -> bool {
        self.crouching
    }

    pub(crate) fn crouch_blend(&self) -> f32 {
        self.crouch_blend
    }

    pub(crate) fn collision_height(&self) -> f32 {
        if self.crouching {
            CROUCH_HEIGHT
        } else {
            PLAYER_HEIGHT
        }
    }

    pub(crate) fn reset_motion(&mut self) {
        self.velocity = Vec3::ZERO;
        self.step_target_y = None;
        self.running = false;
        self.run_deadline_tick = None;
        self.crouching = false;
        self.crouch_blend = 0.0;
    }
}

pub(super) fn walk(
    game_rules: Res<GameRules>,
    world_ticks: Res<WorldTickClock>,
    keys: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    world: Res<VoxelWorld>,
    camera: Single<&GameplayCamera>,
    perspective: Res<CameraPerspective>,
    player: Single<
        (
            &mut Transform,
            &FlightState,
            &SwimmingState,
            &mut GravityState,
            &mut WalkingState,
        ),
        With<PlayerEntity>,
    >,
) {
    let camera = camera.into_inner();
    let (mut transform, flight, swimming, mut gravity, mut walking) = player.into_inner();

    update_crouch_state(
        &keys,
        &keybinds,
        &world,
        &transform,
        flight,
        swimming,
        &mut walking,
    );

    let delta_seconds = world_ticks.delta_seconds(&game_rules);
    if delta_seconds > 0.0 {
        let crouch_target = if walking.crouching { 1.0 } else { 0.0 };
        walking.crouch_blend = approach_scalar(
            walking.crouch_blend,
            crouch_target,
            CROUCH_TRANSITION_SPEED * delta_seconds,
        );
    }

    if flight.active {
        if walking.velocity != Vec3::ZERO {
            walking.velocity = Vec3::ZERO;
        }
        walking.running = false;
        walking.run_deadline_tick = None;
        return;
    }

    update_run_state(&keys, &world_ticks, &mut walking);

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

    let (forward, right) = perspective.horizontal_movement_axes(camera.yaw);
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

    let move_speed = if walking.crouching {
        WALK_SPEED * CROUCH_SPEED_MULTIPLIER
    } else if walking.running {
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
    let player_height = walking.collision_height();
    let protect_edges = walking.crouching && gravity.grounded;
    let velocity = walking.velocity;
    if velocity.x != 0.0
        && !move_horizontal_axis(
            &mut transform,
            &world,
            velocity.x * delta_seconds,
            Axis::X,
            step_up_height,
            &mut walking.step_target_y,
            player_height,
            protect_edges,
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
            player_height,
            protect_edges,
        )
    {
        walking.velocity.z = 0.0;
    }
}

fn update_crouch_state(
    keys: &ButtonInput<KeyCode>,
    keybinds: &Keybinds,
    world: &VoxelWorld,
    transform: &Transform,
    flight: &FlightState,
    swimming: &SwimmingState,
    walking: &mut WalkingState,
) {
    let requested = keys.pressed(keybinds.key_code(KeybindAction::Descend))
        && !flight.active
        && !swimming.active;

    if requested {
        walking.crouching = true;
        return;
    }

    if walking.crouching
        && player_collides_with_height(transform.translation, world, PLAYER_HEIGHT)
    {
        return;
    }

    walking.crouching = false;
}

fn update_run_state(
    keys: &ButtonInput<KeyCode>,
    world_ticks: &WorldTickClock,
    walking: &mut WalkingState,
) {
    if walking.crouching {
        walking.running = false;
        walking.run_deadline_tick = None;
        return;
    }

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
    player_height: f32,
    protect_edges: bool,
) -> bool {
    let visual_y = transform.translation.y;
    if let Some(target_y) = *step_target_y {
        transform.translation.y = target_y;
    }

    let delta = if protect_edges {
        supported_horizontal_delta(transform, world, delta, axis, player_height)
    } else {
        delta
    };
    if delta == 0.0 {
        transform.translation.y = visual_y;
        return false;
    }

    let result = move_axis_with_height(
        transform,
        world,
        delta,
        axis,
        step_up_height,
        player_height,
    );
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

fn supported_horizontal_delta(
    transform: &Transform,
    world: &VoxelWorld,
    delta: f32,
    axis: Axis,
    player_height: f32,
) -> f32 {
    let mut candidate = delta;
    let direction = delta.signum();

    while candidate != 0.0 {
        let mut position = transform.translation;
        match axis {
            Axis::X => position.x += candidate,
            Axis::Z => position.z += candidate,
            Axis::Y => return delta,
        }

        if player_has_ground_support(position, world, player_height) {
            return candidate;
        }

        let next = (candidate.abs() - COLLISION_STEP).max(0.0);
        candidate = if next <= f32::EPSILON {
            0.0
        } else {
            next * direction
        };
    }

    0.0
}

fn approach_step_height(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    if delta.abs() <= max_delta || max_delta <= 0.0 {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

fn approach_scalar(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    if delta.abs() <= max_delta || max_delta <= 0.0 {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

#[cfg(test)]
mod tests {
    use super::{approach_scalar, approach_step_height};

    #[test]
    fn step_height_moves_toward_target_without_overshoot() {
        assert_eq!(approach_step_height(1.0, 2.0, 0.25), 1.25);
        assert_eq!(approach_step_height(1.9, 2.0, 0.25), 2.0);
        assert_eq!(approach_step_height(2.0, 1.0, 0.25), 1.75);
    }

    #[test]
    fn crouch_blend_moves_toward_target_without_overshoot() {
        assert_eq!(approach_scalar(0.0, 1.0, 0.25), 0.25);
        assert_eq!(approach_scalar(0.9, 1.0, 0.25), 1.0);
        assert_eq!(approach_scalar(1.0, 0.0, 0.25), 0.75);
    }
}
