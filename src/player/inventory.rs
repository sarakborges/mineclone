use bevy::prelude::*;

use crate::{
    app::resource_systems::reset_resource,
    content::builtin_ids::{
        DIMENSIONAL_SLICER_ITEM_ID, DIMENSIONAL_SLICER_TARGET_DIMENSION_METADATA_KEY,
        UMBRAL_DIMENSION_ID,
    },
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
        let mut slot = inventory.replace_inventory_item(index, None);
        self.click_item_slot(&mut slot);
        inventory.replace_inventory_item(index, slot);
    }

    pub(crate) fn click_item_slot(&mut self, slot: &mut Option<ItemStack>) {
        let cursor = self.item.take();
        let slot_item = slot.take();

        match (cursor, slot_item) {
            (None, slot_item) => {
                self.item = slot_item;
            }
            (Some(cursor), None) => {
                *slot = Some(cursor);
            }
            (Some(cursor), Some(mut slot_stack)) if slot_stack.can_stack_with(&cursor) => {
                self.item = slot_stack.merge_from(cursor);
                *slot = Some(slot_stack);
            }
            (Some(cursor), Some(slot_stack)) => {
                *slot = Some(cursor);
                self.item = Some(slot_stack);
            }
        }
    }

    pub(crate) fn pick_creative_item(
        &mut self,
        item: &'static str,
        metadata: Option<(&'static str, &'static str)>,
        fill_stack: bool,
    ) {
        let metadata = metadata.or_else(|| {
            (item == DIMENSIONAL_SLICER_ITEM_ID).then_some((
                DIMENSIONAL_SLICER_TARGET_DIMENSION_METADATA_KEY,
                UMBRAL_DIMENSION_ID,
            ))
        });
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

    pub(crate) fn set_stack(&mut self, stack: Option<ItemStack>) {
        self.item = stack;
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

    #[test]
    fn creative_dimensional_slicer_defaults_to_umbral_target() {
        let mut cursor = InventoryCursor::default();

        cursor.pick_creative_item(DIMENSIONAL_SLICER_ITEM_ID, None, false);

        assert_eq!(
            cursor
                .stack()
                .unwrap()
                .metadata()
                .get(DIMENSIONAL_SLICER_TARGET_DIMENSION_METADATA_KEY),
            Some(UMBRAL_DIMENSION_ID)
        );
    }

    #[test]
    fn explicit_dimensional_slicer_target_overrides_creative_default() {
        let mut cursor = InventoryCursor::default();

        cursor.pick_creative_item(
            DIMENSIONAL_SLICER_ITEM_ID,
            Some((
                DIMENSIONAL_SLICER_TARGET_DIMENSION_METADATA_KEY,
                "asteria:overworld",
            )),
            false,
        );

        assert_eq!(
            cursor
                .stack()
                .unwrap()
                .metadata()
                .get(DIMENSIONAL_SLICER_TARGET_DIMENSION_METADATA_KEY),
            Some("asteria:overworld")
        );
    }

    #[test]
    fn generic_slot_click_swaps_cursor_and_slot() {
        let mut cursor = InventoryCursor::default();
        cursor.set_stack(Some(ItemStack::new("asteria:pebble")));
        let mut slot = Some(ItemStack::new("asteria:stick"));

        cursor.click_item_slot(&mut slot);

        assert_eq!(cursor.item(), Some("asteria:stick"));
        assert_eq!(slot.as_ref().map(ItemStack::id), Some("asteria:pebble"));
    }
}
