pub mod camera;
pub mod movement;

use bevy::prelude::*;

use crate::{app::game_state::GameState, voxel::chunk::CHUNK_SIZE};
use camera::GameplayCamera;
use movement::{
    flight::FlightState,
    gravity::GravityState,
    walking::WalkingState,
};

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
    let feet_y = CHUNK_SIZE as f32;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(8.0, feet_y + PLAYER_EYE_HEIGHT, 8.0),
        GameplayCamera::default(),
        WalkingState::default(),
        FlightState::default(),
        GravityState::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
}
