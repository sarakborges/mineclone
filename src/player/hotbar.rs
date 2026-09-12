use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::builtin_ids::{
        DIRT_BLOCK_ID, GLASS_BLOCK_ID, GRASS_BLOCK_ID, LAMP_BLOCK_ID, OAK_LEAF_BLOCK_ID,
        OAK_WOOD_BLOCK_ID, SAND_BLOCK_ID, STONE_BLOCK_ID,
    },
    player::inventory::InventoryState,
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
        let mut slots = [None; HOTBAR_SLOT_COUNT];
        slots[0] = Some(GRASS_BLOCK_ID);
        slots[1] = Some(DIRT_BLOCK_ID);
        slots[2] = Some(STONE_BLOCK_ID);
        slots[3] = Some(LAMP_BLOCK_ID);
        slots[4] = Some(SAND_BLOCK_ID);
        slots[5] = Some(OAK_WOOD_BLOCK_ID);
        slots[6] = Some(OAK_LEAF_BLOCK_ID);
        slots[7] = Some(GLASS_BLOCK_ID);

        Self {
            selected_slot: 0,
            backpack: [None; BACKPACK_SLOT_COUNT],
            slots,
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
        let slot = self
            .slots
            .get_mut(hotbar_index)
            .unwrap_or_else(|| panic!("inventory slot must be between 0 and {}", INVENTORY_SLOT_COUNT - 1));
        std::mem::replace(slot, item)
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
        app.init_resource::<PlayerHotbar>()
            .add_systems(OnEnter(GameState::Gameplay), reset_hotbar_selection)
            .add_systems(
                Update,
                select_hotbar_slot
                    .in_set(PlayerHotbarSet::Selection)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(InventoryState::Closed)),
            );
    }
}

fn reset_hotbar_selection(mut hotbar: ResMut<PlayerHotbar>) {
    hotbar.select(0);
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
