mod scene;

use bevy::prelude::*;

use crate::player::{
    camera::PlayerCameraPlugin,
    hud::PlayerHudPlugin,
    movement::PlayerMovementPlugin,
    PlayerPlugin,
};
use scene::GameplayScenePlugin;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameplayScenePlugin,
            PlayerPlugin,
            PlayerCameraPlugin,
            PlayerMovementPlugin,
            PlayerHudPlugin,
        ));
    }
}
