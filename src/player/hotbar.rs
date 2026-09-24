use std::{cmp::Ordering, io};

use bevy::prelude::*;

use crate::{
    content::{
        block::BlockRegistry, block_id::intern_block_id, item::ItemRegistry,
        item_id::intern_item_id, layer::LayerRegistry, layer_id::intern_layer_id,
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

    pub(crate) fn inventory_item_at(&self, index: usize) -> Option<&'static str> {
        self.inventory_stack_at(index).map(ItemStack::id)
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
        self.backpack.sort_by(|left, right| match (left, right) {
            (Some(left), Some(right)) => left.cmp(right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        });
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

    pub(crate) fn set_selected_item(&mut self, item: Option<&'static str>) {
        self.set_selected_stack(item.map(ItemStack::new));
    }

    pub(crate) fn set_selected_stack(&mut self, item: Option<ItemStack>) {
        self.slots[self.selected_slot] = item;
    }

    fn select(&mut self, slot: usize) {
        assert!(
            slot < HOTBAR_SLOT_COUNT,
            "hotbar slot must be between 0 and 8"
        );
        self.selected_slot = slot;
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
