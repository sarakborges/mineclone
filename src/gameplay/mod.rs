mod pause;

use bevy::prelude::*;

use crate::player::{
    camera::PlayerCameraPlugin, hotbar::PlayerHotbarPlugin, inventory::PlayerInventoryPlugin,
    movement::PlayerMovementPlugin, viewmodel::PlayerViewModelPlugin,
};
use pause::PausePlugin;

pub(crate) struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PlayerInventoryPlugin,
            PausePlugin,
            PlayerHotbarPlugin,
            PlayerViewModelPlugin,
            PlayerCameraPlugin,
            PlayerMovementPlugin,
        ));
    }
}
