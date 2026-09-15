use bevy::prelude::*;

use crate::{
    app::{
        game_state::GameState, pause_state::PauseState, resource_systems::reset_resource,
        state_systems::reset_next_state,
    },
    tools::BrushPaletteState,
};

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
        app.init_state::<InventoryState>()
            .init_resource::<InventoryCursor>()
            .add_systems(
                Update,
                toggle_inventory
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(BrushPaletteState::Closed)),
            )
            .add_systems(
                OnExit(InventoryState::Open),
                reset_resource::<InventoryCursor>,
            )
            .add_systems(
                OnEnter(PauseState::Paused),
                reset_next_state::<InventoryState>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                reset_next_state::<InventoryState>,
            );
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
