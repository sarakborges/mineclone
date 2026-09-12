use bevy::prelude::*;

use crate::app::{game_state::GameState, pause_state::PauseState};

use super::{game_mode::GameMode, hotbar::PlayerHotbar};

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

    fn clear(&mut self) {
        self.item = None;
    }
}

#[derive(Resource, Default)]
pub(crate) struct CreativeInventoryView {
    search_query: String,
    search_focused: bool,
    selected_category: Option<String>,
    scroll_row: usize,
}

impl CreativeInventoryView {
    pub(crate) fn search_query(&self) -> &str {
        &self.search_query
    }

    pub(crate) fn search_focused(&self) -> bool {
        self.search_focused
    }

    pub(crate) fn selected_category(&self) -> Option<&str> {
        self.selected_category.as_deref()
    }

    pub(crate) fn scroll_row(&self) -> usize {
        self.scroll_row
    }

    pub(crate) fn focus_search(&mut self) {
        self.search_focused = true;
    }

    pub(crate) fn blur_search(&mut self) {
        self.search_focused = false;
    }

    pub(crate) fn push_search_text(&mut self, text: &str) {
        let previous_len = self.search_query.len();
        self.search_query
            .extend(text.chars().filter(|character| !character.is_control()));
        if self.search_query.len() != previous_len {
            self.scroll_row = 0;
        }
    }

    pub(crate) fn backspace_search(&mut self) {
        if self.search_query.pop().is_some() {
            self.scroll_row = 0;
        }
    }

    pub(crate) fn select_category(&mut self, category: Option<&str>) {
        let category = category.map(str::to_owned);
        if self.selected_category != category {
            self.selected_category = category;
            self.scroll_row = 0;
        }
    }

    pub(crate) fn set_scroll_row(&mut self, row: usize) {
        self.scroll_row = row;
    }

    fn reset(&mut self) {
        self.search_query.clear();
        self.search_focused = false;
        self.selected_category = None;
        self.scroll_row = 0;
    }
}

pub(crate) struct PlayerInventoryPlugin;

impl Plugin for PlayerInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<InventoryState>()
            .init_resource::<InventoryCursor>()
            .init_resource::<CreativeInventoryView>()
            .add_systems(
                Update,
                toggle_inventory
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                OnExit(InventoryState::Open),
                (discard_cursor_item, reset_creative_inventory_view),
            )
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
    creative_view: Res<CreativeInventoryView>,
    game_mode: Single<&GameMode>,
    mut next_inventory_state: ResMut<NextState<InventoryState>>,
) {
    match inventory_state.get() {
        InventoryState::Open if !game_mode.has_creative_inventory() => {
            next_inventory_state.set(InventoryState::Closed);
        }
        InventoryState::Closed
            if keys.just_pressed(KeyCode::KeyE) && game_mode.has_creative_inventory() =>
        {
            next_inventory_state.set(InventoryState::Open);
        }
        InventoryState::Open if keys.just_pressed(KeyCode::Escape) => {
            next_inventory_state.set(InventoryState::Closed);
        }
        InventoryState::Open
            if keys.just_pressed(KeyCode::KeyE) && !creative_view.search_focused() =>
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

fn reset_creative_inventory_view(mut creative_view: ResMut<CreativeInventoryView>) {
    creative_view.reset();
}
