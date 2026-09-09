use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, WindowFocused},
};

use super::look::MouseLookInputState;

pub(super) fn capture_cursor(
    window: Single<&Window>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut mouse_input: ResMut<MouseLookInputState>,
) {
    set_cursor_capture(window.focused, &mut cursor_options, &mut mouse_input);
}

pub(super) fn release_cursor(
    mut cursor_options: Single<&mut CursorOptions>,
    mut mouse_input: ResMut<MouseLookInputState>,
) {
    set_cursor_capture(false, &mut cursor_options, &mut mouse_input);
}

pub(super) fn handle_window_focus(
    mut focused_events: MessageReader<WindowFocused>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut mouse_input: ResMut<MouseLookInputState>,
) {
    for _event in focused_events.read() {
        // Never request a Locked grab from inside the OS focus transition itself.
        // Alt+Tab can briefly leave the native window in a state where relocking is
        // unsafe. Returning to the game leaves the cursor released until the player
        // clicks the window again.
        set_cursor_capture(false, &mut cursor_options, &mut mouse_input);
    }
}

pub(super) fn handle_cursor_grab(
    window: Single<&Window>,
    mut cursor_options: Single<&mut CursorOptions>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_input: ResMut<MouseLookInputState>,
) {
    if !window.focused || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    set_cursor_capture(true, &mut cursor_options, &mut mouse_input);
}

fn set_cursor_capture(
    captured: bool,
    cursor_options: &mut CursorOptions,
    mouse_input: &mut MouseLookInputState,
) {
    cursor_options.visible = !captured;
    cursor_options.grab_mode = if captured {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    mouse_input.ignore_next_delta = true;
}
