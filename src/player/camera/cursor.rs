use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use super::look::MouseLookInputState;

pub(super) fn capture_cursor(
    mut cursor_options: Single<&mut CursorOptions>,
    mut mouse_input: ResMut<MouseLookInputState>,
) {
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;
    mouse_input.ignore_next_delta = true;
}

pub(super) fn release_cursor(mut cursor_options: Single<&mut CursorOptions>) {
    cursor_options.visible = true;
    cursor_options.grab_mode = CursorGrabMode::None;
}

pub(super) fn handle_cursor_grab(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_input: ResMut<MouseLookInputState>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;
    mouse_input.ignore_next_delta = true;
}
