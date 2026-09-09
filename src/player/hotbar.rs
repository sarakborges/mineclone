use bevy::prelude::*;

use crate::app::{game_state::GameState, pause_state::PauseState};

pub const HOTBAR_SLOT_COUNT: usize = 9;
pub const GRASS_BLOCK_ID: &str = "asteria:grass";
pub const LAMP_BLOCK_ID: &str = "asteria:lamp";

#[derive(Resource)]
pub struct PlayerHotbar {
    selected_slot: usize,
    slots: [Option<&'static str>; HOTBAR_SLOT_COUNT],
}

impl Default for PlayerHotbar {
    fn default() -> Self {
        let mut slots = [None; HOTBAR_SLOT_COUNT];
        slots[0] = Some(GRASS_BLOCK_ID);
        slots[1] = Some(LAMP_BLOCK_ID);

        Self {
            selected_slot: 0,
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

    fn select(&mut self, slot: usize) {
        assert!(slot < HOTBAR_SLOT_COUNT, "hotbar slot must be between 0 and 8");
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
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
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
