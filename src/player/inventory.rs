use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    tools::BrushPaletteState,
    ui::text_input::select_all_pressed,
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

    fn clear(&mut self) {
        self.item = None;
    }
}

#[derive(Resource, Default)]
pub(crate) struct CreativeInventoryView {
    search_query: String,
    search_focused: bool,
    replace_search_on_next_input: bool,
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
        self.replace_search_on_next_input = false;
    }

    pub(crate) fn blur_search(&mut self) {
        self.search_focused = false;
        self.replace_search_on_next_input = false;
    }

    fn select_all_search(&mut self) {
        if self.search_focused {
            self.replace_search_on_next_input = true;
        }
    }

    pub(crate) fn push_search_text(&mut self, text: &str) {
        let filtered = text
            .chars()
            .filter(|character| !character.is_control())
            .collect::<String>();
        if filtered.is_empty() {
            return;
        }

        if self.replace_search_on_next_input {
            self.search_query.clear();
            self.replace_search_on_next_input = false;
        }

        self.search_query.push_str(&filtered);
        self.scroll_row = 0;
    }

    pub(crate) fn backspace_search(&mut self) {
        if self.replace_search_on_next_input {
            if !self.search_query.is_empty() {
                self.search_query.clear();
                self.scroll_row = 0;
            }
            self.replace_search_on_next_input = false;
            return;
        }

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
        self.replace_search_on_next_input = false;
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
                (toggle_inventory, handle_search_select_all)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(BrushPaletteState::Closed)),
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
    mut next_inventory_state: ResMut<NextState<InventoryState>>,
) {
    match inventory_state.get() {
        InventoryState::Closed if keys.just_pressed(KeyCode::KeyE) => {
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

fn handle_search_select_all(
    keys: Res<ButtonInput<KeyCode>>,
    inventory_state: Res<State<InventoryState>>,
    mut creative_view: ResMut<CreativeInventoryView>,
) {
    if *inventory_state.get() == InventoryState::Open && select_all_pressed(&keys) {
        creative_view.select_all_search();
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
