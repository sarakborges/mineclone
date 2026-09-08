pub mod camera;
pub mod hud;
pub mod movement;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use camera::GameplayCamera;
use movement::PlayerMovement;

pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_EYE_HEIGHT: f32 = 1.62;
pub const PLAYER_HALF_WIDTH: f32 = 0.3;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_player);
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(8.0, PLAYER_EYE_HEIGHT + 1.0, 8.0),
        GameplayCamera::default(),
        PlayerMovement::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
}
