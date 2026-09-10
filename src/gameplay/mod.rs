mod pause;

use bevy::prelude::*;

use crate::player::{
    PlayerPlugin, camera::PlayerCameraPlugin, hotbar::PlayerHotbarPlugin,
    movement::PlayerMovementPlugin, viewmodel::PlayerViewModelPlugin,
};
use pause::PausePlugin;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PausePlugin,
            PlayerPlugin,
            PlayerHotbarPlugin,
            PlayerViewModelPlugin,
            PlayerCameraPlugin,
            PlayerMovementPlugin,
        ));
    }
}
