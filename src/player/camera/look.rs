use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::app::pause_state::PauseState;

use super::GameplayCamera;

const MOUSE_SENSITIVITY: f32 = 0.003;
const MAX_PITCH: f32 = 1.54;

#[derive(Resource, Default)]
pub(super) struct MouseLookInputState {
    pub(super) ignore_next_delta: bool,
}

pub(super) fn drain_or_apply_mouse_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    pause_state: Res<State<PauseState>>,
    window: Single<&Window>,
    cursor_options: Single<&CursorOptions>,
    mut input_state: ResMut<MouseLookInputState>,
    mut camera: Single<(&mut Transform, &mut GameplayCamera)>,
) {
    let delta = mouse_motion.read().map(|motion| motion.delta).sum::<Vec2>();

    if !window.focused
        || *pause_state.get() == PauseState::Paused
        || cursor_options.grab_mode == CursorGrabMode::None
        || delta == Vec2::ZERO
    {
        return;
    }

    if input_state.ignore_next_delta {
        input_state.ignore_next_delta = false;
        return;
    }

    camera.1.yaw -= delta.x * MOUSE_SENSITIVITY;
    camera.1.pitch = (camera.1.pitch - delta.y * MOUSE_SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);
    camera.0.rotation = Quat::from_euler(EulerRot::YXZ, camera.1.yaw, camera.1.pitch, 0.0);
}
