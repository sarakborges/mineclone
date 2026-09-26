pub(crate) mod availability;
pub(crate) mod modal;
pub(crate) mod random;
mod pause;

use bevy::prelude::*;

use crate::player::{
    camera::PlayerCameraPlugin, character_info::PlayerCharacterInfoPlugin,
    hotbar::PlayerHotbarPlugin, inventory::PlayerInventoryPlugin, model::PlayerModelPlugin,
    movement::PlayerMovementPlugin, viewmodel::PlayerViewModelPlugin,
};
use modal::GameplayModalPlugin;
use pause::PausePlugin;

pub(crate) struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameplayModalPlugin,
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
