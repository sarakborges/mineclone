use bevy::prelude::*;

use crate::{
    app::resource_systems::reset_resource,
    gameplay::modal::GameplayModalState,
};

use super::hotbar::PlayerHotbar;

#[derive(Resource, Default)]
pub(crate) struct InventoryCursor {
    item: Option<&'static str>,
}

impl InventoryCursor {
    pub(crate) fn item(&self) -> Option<&'static str> {
        self.item
    }

    pub(crate) fn click_slot(&mut self, inventory: &mut PlayerHotbar, index: usize) {
        self.item = inventory.replace_inventory_item(index, self.item);
    }

    pub(crate) fn pick_creative_item(&mut self, item: &'static str) {
        self.item = Some(item);
    }

    pub(crate) fn discard(&mut self) {
        self.item = None;
    }
}

pub(crate) struct PlayerInventoryPlugin;

impl Plugin for PlayerInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryCursor>().add_systems(
            OnExit(GameplayModalState::Inventory),
            reset_resource::<InventoryCursor>,
        );
    }
}
