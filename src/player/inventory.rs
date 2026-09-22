use bevy::{ecs::system::SystemParam, input_focus::InputFocus, prelude::*};

use crate::{
    app::{
        keybinds::{KeybindAction, Keybinds},
        game_state::GameState, pause_state::PauseState, resource_systems::reset_resource,
        state_systems::reset_next_state,
    },
    hud::chat::ChatState,
    player::character_info::CharacterInfoState,
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
                    .run_if(in_state(PauseState::Running)),
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

#[derive(SystemParam)]
struct InventoryModalInput<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    keybinds: Res<'w, Keybinds>,
    inventory_state: Res<'w, State<InventoryState>>,
    chat: Res<'w, ChatState>,
    focus: Res<'w, InputFocus>,
    next_inventory_state: ResMut<'w, NextState<InventoryState>>,
    next_character_info: ResMut<'w, NextState<CharacterInfoState>>,
    next_brush_palette: ResMut<'w, NextState<BrushPaletteState>>,
}

fn toggle_inventory(mut input: InventoryModalInput) {
    if input.chat.is_open() || input.focus.get().is_some() {
        return;
    }
    match input.inventory_state.get() {
        InventoryState::Closed
            if input
                .keys
                .just_pressed(input.keybinds.key_code(KeybindAction::Inventory)) =>
        {
            input
                .next_character_info
                .set(CharacterInfoState::Closed);
            input.next_brush_palette.set(BrushPaletteState::Closed);
            input.next_inventory_state.set(InventoryState::Open);
        }
        InventoryState::Open if input.keys.just_pressed(KeyCode::Escape) => {
            input.next_inventory_state.set(InventoryState::Closed);
        }
        _ => {}
    }
}
