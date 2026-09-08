use bevy::prelude::*;

use crate::app::{game_state::GameState, pause_state::PauseState};
use cursor::{capture_cursor, handle_cursor_grab, release_cursor};
use look::{drain_or_apply_mouse_look, MouseLookInputState};

mod cursor;
mod look;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseLookInputState>()
            .add_systems(OnEnter(GameState::Gameplay), capture_cursor)
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
                handle_cursor_grab
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                Update,
                drain_or_apply_mouse_look.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component, Default)]
pub struct GameplayCamera {
    pub yaw: f32,
    pub pitch: f32,
}
