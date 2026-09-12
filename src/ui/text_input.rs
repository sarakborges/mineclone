use bevy::prelude::*;

pub(crate) fn select_all_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
        && keys.just_pressed(KeyCode::KeyA)
}
