use bevy::prelude::*;

use crate::app::{game_state::GameState, pause_state::PauseState};

use super::hotbar::PlayerHotbar;

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum InventoryState {
    #[default]
    Closed,
    Open,
}

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

    fn clear(&mut self) {
        self.item = None;
    }
}

pub(crate) struct PlayerInventoryPlugin;

impl Plugin for PlayerInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<InventoryState>()
            .init_resource::<InventoryCursor>()
            .add_systems(
                Update,
                toggle_inventory
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(OnExit(InventoryState::Open), discard_cursor_item)
            .add_systems(
                OnEnter(PauseState::Paused),
                close_inventory.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(OnExit(GameState::Gameplay), close_inventory);
    }
}

fn toggle_inventory(
    keys: Res<ButtonInput<KeyCode>>,
    inventory_state: Res<State<InventoryState>>,
    mut next_inventory_state: ResMut<NextState<InventoryState>>,
) {
    match inventory_state.get() {
        InventoryState::Closed if keys.just_pressed(KeyCode::KeyE) => {
            next_inventory_state.set(InventoryState::Open);
        }
        InventoryState::Open
            if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Escape) =>
        {
            next_inventory_state.set(InventoryState::Closed);
        }
        _ => {}
    }
}

fn close_inventory(mut next_inventory_state: ResMut<NextState<InventoryState>>) {
    next_inventory_state.set(InventoryState::Closed);
}

fn discard_cursor_item(mut cursor: ResMut<InventoryCursor>) {
    cursor.clear();
}
