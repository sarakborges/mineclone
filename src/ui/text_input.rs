use bevy::prelude::*;

pub(crate) fn select_all_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    let control_pressed =
        keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let control_just_pressed =
        keys.just_pressed(KeyCode::ControlLeft) || keys.just_pressed(KeyCode::ControlRight);

    (control_pressed && keys.just_pressed(KeyCode::KeyA))
        || (keys.pressed(KeyCode::KeyA) && control_just_pressed)
}
