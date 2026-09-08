mod pause;

use bevy::prelude::*;

use crate::player::{
    camera::PlayerCameraPlugin,
    hotbar::PlayerHotbarPlugin,
    movement::PlayerMovementPlugin,
    PlayerPlugin,
};
use pause::PausePlugin;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PausePlugin,
            PlayerPlugin,
            PlayerHotbarPlugin,
            PlayerCameraPlugin,
            PlayerMovementPlugin,
        ));
    }
}
