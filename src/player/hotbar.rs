use std::io;

use bevy::prelude::*;

use crate::{
    content::{
        block::BlockRegistry, block_id::intern_block_id, tool::ToolRegistry,
        tool_id::intern_tool_id,
    },
    gameplay::availability::world_interaction_available,
};

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
    backpack: [Option<&'static str>; BACKPACK_SLOT_COUNT],
    slots: [Option<&'static str>; HOTBAR_SLOT_COUNT],
}

impl Default for PlayerHotbar {
    fn default() -> Self {
        Self {
            selected_slot: 0,
            backpack: [None; BACKPACK_SLOT_COUNT],
            slots: [None; HOTBAR_SLOT_COUNT],
        }
    }
}

impl PlayerHotbar {
    pub fn selected_slot(&self) -> usize {
        self.selected_slot
    }

    pub fn item_at(&self, slot: usize) -> Option<&'static str> {
        self.slots.get(slot).copied().flatten()
    }

    pub(crate) fn inventory_item_at(&self, index: usize) -> Option<&'static str> {
        if index < BACKPACK_SLOT_COUNT {
            return self.backpack[index];
        }

        self.item_at(index - HOTBAR_INVENTORY_OFFSET)
    }

    pub(crate) fn replace_inventory_item(
        &mut self,
        index: usize,
        item: Option<&'static str>,
    ) -> Option<&'static str> {
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

    pub(crate) fn saved_items(&self) -> Vec<Option<String>> {
        self.backpack
            .iter()
            .chain(self.slots.iter())
            .map(|item| item.map(str::to_owned))
            .collect()
    }

    pub(crate) fn from_saved_items_and_selection(
        items: &[Option<String>],
        selected_slot: usize,
        blocks: &BlockRegistry,
        tools: &ToolRegistry,
    ) -> io::Result<Self> {
        if items.len() != INVENTORY_SLOT_COUNT {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid inventory length"));
        }
        if selected_slot >= HOTBAR_SLOT_COUNT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid selected hotbar slot",
            ));
        }

        let mut restored = Self::default();
        for (index, item) in items.iter().enumerate() {
            let resolved = match item {
                None => None,
                Some(id) if blocks.get(id).is_some() => Some(intern_block_id(id)),
                Some(id) if tools.get(id).is_some() => Some(intern_tool_id(id)),
                Some(id) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown inventory item ID: {id}"),
                    ));
                }
            };

            if index < BACKPACK_SLOT_COUNT {
                restored.backpack[index] = resolved;
            } else {
                restored.slots[index - HOTBAR_INVENTORY_OFFSET] = resolved;
            }
        }
        restored.selected_slot = selected_slot;
        Ok(restored)
    }

    pub(crate) fn restore_items_and_selection(
        &mut self,
        items: &[Option<String>],
        selected_slot: usize,
        blocks: &BlockRegistry,
        tools: &ToolRegistry,
    ) -> io::Result<()> {
        *self = Self::from_saved_items_and_selection(
            items,
            selected_slot,
            blocks,
            tools,
        )?;
        Ok(())
    }

    pub(crate) fn set_selected_item(&mut self, item: Option<&'static str>) {
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
