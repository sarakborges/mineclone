use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    gameplay::availability::world_interaction_available,
    player::inventory::InventoryState,
    tools::BrushPaletteState,
};
use cursor::{capture_cursor, handle_cursor_grab, handle_window_focus, release_cursor};
use look::{MouseLookInputState, drain_or_apply_mouse_look};

mod cursor;
pub(crate) mod look;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseLookInputState>()
            .add_systems(
                OnEnter(GameState::Gameplay),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(OnExit(GameState::Gameplay), release_cursor)
            .add_systems(
                OnEnter(PauseState::Paused),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(InventoryState::Open),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(InventoryState::Closed),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Open),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Closed),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(
                Update,
                handle_window_focus.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                handle_cursor_grab.run_if(world_interaction_available),
            )
            .add_systems(
                Update,
                drain_or_apply_mouse_look.run_if(in_state(GameState::Gameplay)),
            );
    }
}

pub(crate) const MAX_CAMERA_PITCH: f32 = 1.54;

#[derive(Component, Default, Clone, Copy)]
pub struct GameplayCamera {
    pub yaw: f32,
    pub pitch: f32,
}

impl GameplayCamera {
    pub(crate) fn restored(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch: pitch.clamp(-MAX_CAMERA_PITCH, MAX_CAMERA_PITCH) }
    }

    pub(crate) fn rotation(self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }
}
