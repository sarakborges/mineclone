use std::io;

use bevy::prelude::*;

use crate::{
    content::{
        block::BlockRegistry, block_id::intern_block_id, item::ItemRegistry,
        item_id::intern_item_id, layer::LayerRegistry, layer_id::intern_layer_id,
        object::ObjectRegistry, object_id::intern_object_id,
        tool::ToolRegistry, tool_id::intern_tool_id,
    },
    gameplay::availability::world_interaction_available,
};

use super::item_stack::{ItemStack, SavedItemStack};

pub const BACKPACK_SLOT_COUNT: usize = 27;
pub const HOTBAR_SLOT_COUNT: usize = 9;
pub const INVENTORY_SLOT_COUNT: usize = BACKPACK_SLOT_COUNT + HOTBAR_SLOT_COUNT;
pub const HOTBAR_INVENTORY_OFFSET: usize = BACKPACK_SLOT_COUNT;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PlayerHotbarSet {
    Selection,
}

#[derive(Resource)]
pub struct PlayerHotbar {
    selected_slot: usize,
    backpack: [Option<ItemStack>; BACKPACK_SLOT_COUNT],
    slots: [Option<ItemStack>; HOTBAR_SLOT_COUNT],
}

impl Default for PlayerHotbar {
    fn default() -> Self {
        Self {
            selected_slot: 0,
            backpack: std::array::from_fn(|_| None),
            slots: std::array::from_fn(|_| None),
        }
    }
}

impl PlayerHotbar {
    pub fn selected_slot(&self) -> usize {
        self.selected_slot
    }

    pub fn item_at(&self, slot: usize) -> Option<&'static str> {
        self.stack_at(slot).map(ItemStack::id)
    }

    pub(crate) fn stack_at(&self, slot: usize) -> Option<&ItemStack> {
        self.slots.get(slot).and_then(Option::as_ref)
    }

    pub(crate) fn inventory_stack_at(&self, index: usize) -> Option<&ItemStack> {
        if index < BACKPACK_SLOT_COUNT {
            return self.backpack[index].as_ref();
        }

        self.stack_at(index - HOTBAR_INVENTORY_OFFSET)
    }

    pub(crate) fn replace_inventory_item(
        &mut self,
        index: usize,
        item: Option<ItemStack>,
    ) -> Option<ItemStack> {
        if index < BACKPACK_SLOT_COUNT {
            return std::mem::replace(&mut self.backpack[index], item);
        }

        let hotbar_index = index - HOTBAR_INVENTORY_OFFSET;
        let slot = self.slots.get_mut(hotbar_index).unwrap_or_else(|| {
            panic!(
                "inventory slot must be between 0 and {}",
                INVENTORY_SLOT_COUNT - 1
            )
        });
        std::mem::replace(slot, item)
    }

    pub(crate) fn sort_backpack_by_id(&mut self) {
        let mut stacks = self
            .backpack
            .iter_mut()
            .filter_map(Option::take)
            .collect::<Vec<_>>();
        stacks.sort_by(ItemStack::stacking_cmp);

        let mut compacted: Vec<ItemStack> = Vec::with_capacity(stacks.len());
        for stack in stacks {
            let remainder = if let Some(last) = compacted.last_mut()
                && last.can_stack_with(&stack)
            {
                last.merge_from(stack)
            } else {
                Some(stack)
            };
            if let Some(remainder) = remainder {
                compacted.push(remainder);
            }
        }

        for (slot, stack) in self.backpack.iter_mut().zip(compacted) {
            *slot = Some(stack);
        }
    }

    pub(crate) fn saved_items(&self) -> Vec<Option<SavedItemStack>> {
        self.backpack
            .iter()
            .chain(self.slots.iter())
            .map(|item| item.as_ref().map(ItemStack::saved))
            .collect()
    }

    pub(crate) fn from_saved_items_and_selection(
        saved_items: &[Option<SavedItemStack>],
        selected_slot: usize,
        items: &ItemRegistry,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        objects: &ObjectRegistry,
        tools: &ToolRegistry,
    ) -> io::Result<Self> {
        if saved_items.len() != INVENTORY_SLOT_COUNT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid inventory length",
            ));
        }
        if selected_slot >= HOTBAR_SLOT_COUNT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid selected hotbar slot",
            ));
        }

        let mut restored = Self::default();
        for (index, item) in saved_items.iter().enumerate() {
            let resolved = item
                .as_ref()
                .map(|saved| {
                    saved.restore(|id| {
                        if items.get(id).is_some() {
                            Some(intern_item_id(id))
                        } else if blocks.get(id).is_some() {
                            Some(intern_block_id(id))
                        } else if layers.get(id).is_some() {
                            Some(intern_layer_id(id))
                        } else if objects.get(id).is_some() {
                            Some(intern_object_id(id))
                        } else if tools.get(id).is_some() {
                            Some(intern_tool_id(id))
                        } else {
                            None
                        }
                    })
                })
                .transpose()?;

            if index < BACKPACK_SLOT_COUNT {
                restored.backpack[index] = resolved;
            } else {
                restored.slots[index - HOTBAR_INVENTORY_OFFSET] = resolved;
            }
        }
        restored.selected_slot = selected_slot;
        Ok(restored)
    }

    pub(crate) fn set_selected_stack(&mut self, item: Option<ItemStack>) {
        self.slots[self.selected_slot] = item;
    }

    pub(crate) fn take_selected_stack(&mut self) -> Option<ItemStack> {
        self.slots[self.selected_slot].take()
    }

    pub(crate) fn consume_selected_item(&mut self) -> bool {
        let slot = &mut self.slots[self.selected_slot];
        let Some(quantity) = slot.as_ref().map(ItemStack::quantity) else {
            return false;
        };

        if quantity == 1 {
            *slot = None;
        } else {
            slot.as_mut()
                .expect("selected stack must still exist")
                .decrement_quantity();
        }
        true
    }

    pub(crate) fn try_insert_stack(&mut self, stack: ItemStack) -> Result<(), ItemStack> {
        let mut remaining = Some(stack);
        merge_into_existing(&mut self.slots, &mut remaining);
        merge_into_existing(&mut self.backpack, &mut remaining);
        let Some(stack) = remaining else {
            return Ok(());
        };

        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(stack);
            return Ok(());
        }
        if let Some(slot) = self.backpack.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(stack);
            return Ok(());
        }
        Err(stack)
    }

    fn select(&mut self, slot: usize) {
        assert!(
            slot < HOTBAR_SLOT_COUNT,
            "hotbar slot must be between 0 and 8"
        );
        self.selected_slot = slot;
    }
}

fn merge_into_existing(slots: &mut [Option<ItemStack>], remaining: &mut Option<ItemStack>) {
    for existing in slots.iter_mut().flatten() {
        let Some(incoming) = remaining.as_ref() else {
            break;
        };
        if !existing.can_stack_with(incoming) {
            continue;
        }
        let incoming = remaining
            .take()
            .expect("remaining stack must exist after compatibility check");
        *remaining = existing.merge_from(incoming);
    }
}

pub struct PlayerHotbarPlugin;

impl Plugin for PlayerHotbarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerHotbar>().add_systems(
            Update,
            select_hotbar_slot
                .in_set(PlayerHotbarSet::Selection)
                .run_if(world_interaction_available),
        );
    }
}

fn select_hotbar_slot(keys: Res<ButtonInput<KeyCode>>, mut hotbar: ResMut<PlayerHotbar>) {
    let bindings = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];

    for (slot, key) in bindings.into_iter().enumerate() {
        if keys.just_pressed(key) {
            hotbar.select(slot);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consuming_selected_stack_decrements_then_clears_the_slot() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.set_selected_stack(Some(ItemStack::new("asteria:stone").with_quantity(2)));

        assert!(hotbar.consume_selected_item());
        assert_eq!(hotbar.stack_at(hotbar.selected_slot()).unwrap().quantity(), 1);

        assert!(hotbar.consume_selected_item());
        assert!(hotbar.stack_at(hotbar.selected_slot()).is_none());
        assert!(!hotbar.consume_selected_item());
    }
}
