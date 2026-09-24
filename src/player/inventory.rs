use bevy::prelude::*;

use crate::{
    app::resource_systems::reset_resource,
    gameplay::modal::GameplayModalState,
};

use super::{
    hotbar::PlayerHotbar,
    item_stack::ItemStack,
};

#[derive(Resource, Default)]
pub(crate) struct InventoryCursor {
    item: Option<ItemStack>,
}

impl InventoryCursor {
    pub(crate) fn item(&self) -> Option<&'static str> {
        self.item.as_ref().map(ItemStack::id)
    }

    pub(crate) fn stack(&self) -> Option<&ItemStack> {
        self.item.as_ref()
    }

    pub(crate) fn click_slot(&mut self, inventory: &mut PlayerHotbar, index: usize) {
        let cursor = self.item.take();
        let slot = inventory.replace_inventory_item(index, None);

        match (cursor, slot) {
            (None, slot) => {
                self.item = slot;
            }
            (Some(cursor), None) => {
                inventory.replace_inventory_item(index, Some(cursor));
            }
            (Some(cursor), Some(mut slot)) if slot.can_stack_with(&cursor) => {
                self.item = slot.merge_from(cursor);
                inventory.replace_inventory_item(index, Some(slot));
            }
            (Some(cursor), Some(slot)) => {
                inventory.replace_inventory_item(index, Some(cursor));
                self.item = Some(slot);
            }
        }
    }

    pub(crate) fn pick_creative_item(&mut self, item: &'static str) {
        self.item = Some(ItemStack::new(item));
    }

    pub(crate) fn take_stack(&mut self) -> Option<ItemStack> {
        self.item.take()
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
