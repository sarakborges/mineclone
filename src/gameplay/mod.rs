pub(crate) mod availability;
mod pause;

use bevy::prelude::*;

use crate::player::{
    camera::PlayerCameraPlugin, character_info::PlayerCharacterInfoPlugin,
    hotbar::PlayerHotbarPlugin, inventory::PlayerInventoryPlugin, model::PlayerModelPlugin,
    movement::PlayerMovementPlugin, viewmodel::PlayerViewModelPlugin,
};
use pause::PausePlugin;

pub(crate) struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PlayerInventoryPlugin,
            PlayerCharacterInfoPlugin,
            PausePlugin,
            PlayerHotbarPlugin,
            PlayerModelPlugin,
            PlayerViewModelPlugin,
            PlayerCameraPlugin,
            PlayerMovementPlugin,
        ));
    }
}
