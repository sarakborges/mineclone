mod interaction;
mod layout;
mod search_style;
mod state;
mod sync;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    player::inventory::InventoryState,
};

use interaction::{
    handle_category_clicks, handle_creative_scroll, handle_creative_slot_clicks,
    handle_empty_inventory_click, handle_inventory_close_shortcut, handle_inventory_trash_clicks,
    handle_search_focus, handle_search_input, handle_slot_clicks,
    remember_creative_scroll_positions, sync_search_focus,
};
use search_style::{
    focus_inventory_search_frame, frame_inventory_search_field, style_inventory_search_field,
};
use state::{CreativeInventoryUiDirty, CreativeInventoryView, CreativeScrollState};
use sync::{
    rebuild_inventory_when_changed, spawn_inventory, style_category_buttons, style_creative_slots,
    style_inventory_slots, style_inventory_trash_button, style_search_bar,
    sync_inventory_cursor_icon, sync_inventory_slot_contents, update_cursor_icon_position,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InventoryHudSet {
    Input,
    Sync,
    Style,
}

pub(super) struct InventoryHudPlugin;

impl Plugin for InventoryHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreativeInventoryView>()
            .init_resource::<CreativeScrollState>()
            .init_resource::<CreativeInventoryUiDirty>()
            .configure_sets(
                Update,
                (
                    InventoryHudSet::Input,
                    InventoryHudSet::Sync,
                    InventoryHudSet::Style,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(InventoryState::Open),
                spawn_inventory.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(InventoryState::Open),
                (
                    reset_resource::<CreativeInventoryView>,
                    reset_resource::<CreativeScrollState>,
                    reset_resource::<CreativeInventoryUiDirty>,
                ),
            )
            .add_systems(
                Update,
                (
                    remember_creative_scroll_positions,
                    handle_search_focus,
                    focus_inventory_search_frame,
                    handle_inventory_close_shortcut,
                    handle_search_input,
                    handle_category_clicks,
                    handle_creative_scroll,
                    handle_creative_slot_clicks,
                    handle_slot_clicks,
                    handle_inventory_trash_clicks,
                    handle_empty_inventory_click,
                    sync_search_focus,
                )
                    .chain()
                    .in_set(InventoryHudSet::Input)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(InventoryState::Open)),
            )
            .add_systems(
                Update,
                (
                    sync_inventory_cursor_icon,
                    sync_inventory_slot_contents,
                    rebuild_inventory_when_changed,
                )
                    .chain()
                    .in_set(InventoryHudSet::Sync)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(InventoryState::Open)),
            )
            .add_systems(
                Update,
                (
                    style_search_bar,
                    frame_inventory_search_field,
                    style_inventory_search_field,
                    style_category_buttons,
                    style_creative_slots,
                    style_inventory_slots,
                    style_inventory_trash_button,
                    update_cursor_icon_position,
                )
                    .chain()
                    .in_set(InventoryHudSet::Style)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(InventoryState::Open)),
            );
    }
}
