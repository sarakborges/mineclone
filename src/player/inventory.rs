use bevy::prelude::*;

use crate::{
    app::resource_systems::reset_resource,
    gameplay::modal::GameplayModalState,
};

use super::{
    hotbar::PlayerHotbar,
    item_stack::{ItemStack, MAX_STACK_SIZE},
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

    pub(crate) fn pick_creative_item(
        &mut self,
        item: &'static str,
        metadata: Option<(&'static str, &'static str)>,
        fill_stack: bool,
    ) {
        let mut creative_stack = ItemStack::new(item);
        if let Some((key, value)) = metadata {
            creative_stack = creative_stack.with_metadata(key, value);
        }
        if fill_stack {
            creative_stack = creative_stack.with_quantity(MAX_STACK_SIZE);
        }

        let Some(mut held_stack) = self.item.take() else {
            self.item = Some(creative_stack);
            return;
        };

        if !held_stack.can_stack_with(&creative_stack) {
            return;
        }

        let _ = held_stack.merge_from(creative_stack);
        self.item = Some(held_stack);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creative_click_adds_one_to_matching_cursor_stack() {
        let mut cursor = InventoryCursor::default();

        cursor.pick_creative_item("asteria:pebble", None, false);
        cursor.pick_creative_item("asteria:pebble", None, false);

        assert_eq!(cursor.stack().unwrap().quantity(), 2);
    }

    #[test]
    fn creative_shift_click_fills_empty_cursor_stack() {
        let mut cursor = InventoryCursor::default();

        cursor.pick_creative_item("asteria:pebble", None, true);

        assert_eq!(cursor.stack().unwrap().quantity(), MAX_STACK_SIZE);
    }

    #[test]
    fn creative_shift_click_fills_matching_cursor_stack() {
        let mut cursor = InventoryCursor::default();

        cursor.pick_creative_item("asteria:pebble", None, false);
        cursor.pick_creative_item("asteria:pebble", None, true);

        assert_eq!(cursor.stack().unwrap().quantity(), MAX_STACK_SIZE);
    }

    #[test]
    fn creative_click_on_different_item_discards_cursor_stack() {
        let mut cursor = InventoryCursor::default();

        cursor.pick_creative_item("asteria:pebble", None, false);
        cursor.pick_creative_item("asteria:stick", None, false);

        assert!(cursor.stack().is_none());
    }

    #[test]
    fn creative_click_treats_metadata_variants_as_different_items() {
        let mut cursor = InventoryCursor::default();

        cursor.pick_creative_item(
            "asteria:bucket",
            Some(("contained_fluid", "asteria:water")),
            false,
        );
        cursor.pick_creative_item(
            "asteria:bucket",
            Some(("contained_fluid", "asteria:lava")),
            false,
        );

        assert!(cursor.stack().is_none());
    }
}
