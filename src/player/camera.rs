use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::app::{game_state::GameState, pause_state::PauseState};

const MOUSE_SENSITIVITY: f32 = 0.003;
const MAX_PITCH: f32 = 1.54;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), capture_cursor)
            .add_systems(OnExit(GameState::Gameplay), release_cursor)
            .add_systems(
                OnEnter(PauseState::Paused),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                capture_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                (handle_cursor_grab, look_with_mouse)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            );
    }
}

#[derive(Component, Default)]
pub struct GameplayCamera {
    pub yaw: f32,
    pub pitch: f32,
}

fn capture_cursor(mut cursor_options: Single<&mut CursorOptions>) {
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;
}

fn release_cursor(mut cursor_options: Single<&mut CursorOptions>) {
    cursor_options.visible = true;
    cursor_options.grab_mode = CursorGrabMode::None;
}

fn handle_cursor_grab(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    if mouse_buttons.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
}

fn look_with_mouse(
    mut mouse_motion: MessageReader<MouseMotion>,
    cursor_options: Single<&CursorOptions>,
    mut camera: Single<(&mut Transform, &mut GameplayCamera)>,
) {
    if cursor_options.grab_mode == CursorGrabMode::None {
        return;
    }

    let delta = mouse_motion.read().map(|motion| motion.delta).sum::<Vec2>();

    if delta == Vec2::ZERO {
        return;
    }

    camera.1.yaw -= delta.x * MOUSE_SENSITIVITY;
    camera.1.pitch = (camera.1.pitch - delta.y * MOUSE_SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);
    camera.0.rotation = Quat::from_euler(EulerRot::YXZ, camera.1.yaw, camera.1.pitch, 0.0);
}
