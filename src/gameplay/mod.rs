pub(crate) mod availability;
pub(crate) mod modal;
pub(crate) mod random;
pub(crate) mod storage_box;
mod pause;
mod world_crafting;

use bevy::prelude::*;

use crate::player::{
    camera::PlayerCameraPlugin, character_info::PlayerCharacterInfoPlugin,
    hotbar::PlayerHotbarPlugin, inventory::PlayerInventoryPlugin, model::PlayerModelPlugin,
    movement::PlayerMovementPlugin, viewmodel::PlayerViewModelPlugin,
};
use modal::GameplayModalPlugin;
use pause::PausePlugin;
use storage_box::StorageBoxPlugin;
use world_crafting::WorldCraftingPlugin;

pub(crate) struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameplayModalPlugin,
            StorageBoxPlugin,
            PlayerInventoryPlugin,
            PlayerCharacterInfoPlugin,
            PausePlugin,
            PlayerHotbarPlugin,
            PlayerModelPlugin,
            PlayerViewModelPlugin,
            PlayerCameraPlugin,
            PlayerMovementPlugin,
            WorldCraftingPlugin,
        ));
    }
}
