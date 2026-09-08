use bevy::prelude::*;

use crate::{
    gameplay_scene::GameplayScenePlugin,
    player::PlayerPlugin,
    player_camera::PlayerCameraPlugin,
    player_movement::PlayerMovementPlugin,
};

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameplayScenePlugin,
            PlayerPlugin,
            PlayerCameraPlugin,
            PlayerMovementPlugin,
        ));
    }
}
