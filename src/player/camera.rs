use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    player::inventory::InventoryState,
};
use cursor::{capture_cursor, handle_cursor_grab, handle_window_focus, release_cursor};
use look::{MouseLookInputState, drain_or_apply_mouse_look};

mod cursor;
mod look;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseLookInputState>()
            .add_systems(
                OnEnter(GameState::Gameplay),
                capture_cursor.run_if(in_state(InventoryState::Closed)),
            )
            .add_systems(OnExit(GameState::Gameplay), release_cursor)
            .add_systems(
                OnEnter(PauseState::Paused),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                capture_cursor
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(InventoryState::Closed)),
            )
            .add_systems(
                OnEnter(InventoryState::Open),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(InventoryState::Closed),
                capture_cursor
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                Update,
                handle_window_focus.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                handle_cursor_grab
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(InventoryState::Closed)),
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
