mod interaction;
mod layout;
mod search_style;
mod state;
mod sync;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::inventory_category::InventoryCategoryRegistry,
    localization::UiLocalization,
    gameplay::modal::GameplayModalState,
    player::inventory::InventoryCursor,
};

use crate::hud::block_icon::BlockIconMaterial;

use interaction::{
    handle_category_clicks, handle_creative_scroll, handle_creative_slot_clicks,
    handle_empty_inventory_click, handle_inventory_close_shortcut, handle_inventory_sort_clicks,
    handle_inventory_trash_clicks, handle_player_search_focus, handle_player_search_input,
    handle_search_focus, handle_search_input, handle_slot_clicks, remember_creative_scroll_positions,
    sync_player_search_focus, sync_search_focus,
};
use search_style::{
    focus_inventory_search_frame, frame_inventory_search_field, style_inventory_search_field,
    style_player_inventory_search_field,
};
use layout::spawn_character_info_inventory;
use state::{
    CreativeInventoryUiDirty, CreativeInventoryView, CreativeScrollState, PlayerInventoryView,
};
use sync::{
    InventoryItemContent, InventoryPanelState, rebuild_inventory_when_changed, spawn_inventory,
    style_category_buttons, style_creative_slots,
    style_inventory_slots, style_inventory_trash_button, style_search_bar,
    sync_inventory_cursor_icon, sync_inventory_item_tooltip, sync_inventory_slot_contents,
    sync_inventory_sort_tooltip,
    update_cursor_icon_position,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InventoryHudSet {
    Input,
    Sync,
    Style,
}

#[derive(Component)]
pub(super) struct CharacterInfoInventoryRoot;

pub(super) const INVENTORY_SLOT_SIZE: f32 = state::SLOT_SIZE;
pub(super) const INVENTORY_SLOT_GAP: f32 = state::SLOT_GAP;
pub(super) const INVENTORY_PANEL_BORDER_WIDTH: f32 = state::PANEL_BORDER_WIDTH;

#[derive(SystemParam)]
pub(super) struct CharacterInfoInventorySpawn<'w, 's> {
    content: InventoryItemContent<'w>,
    categories: Res<'w, InventoryCategoryRegistry>,
    localization: Res<'w, UiLocalization>,
    panel: InventoryPanelState<'w, 's>,
    window: Single<'w, 's, &'static Window>,
    icon_materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

impl CharacterInfoInventorySpawn<'_, '_> {
    pub(super) fn spawn(&mut self, root: &mut ChildSpawnerCommands) {
        let mut items = self
            .content
            .view(self.panel.player_position(), &mut self.icon_materials);
        let layout = self.panel.layout(
            &self.categories,
            &self.localization,
            self.window.cursor_position(),
        );

        spawn_character_info_inventory(root, &layout, &mut items);
    }
}

pub(super) struct InventoryHudPlugin;

impl Plugin for InventoryHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreativeInventoryView>()
            .init_resource::<PlayerInventoryView>()
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
                OnEnter(GameplayModalState::Inventory),
                spawn_inventory.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameplayModalState::Inventory),
                (
                    reset_resource::<CreativeInventoryView>,
                    reset_resource::<PlayerInventoryView>,
                    reset_resource::<CreativeScrollState>,
                    reset_resource::<CreativeInventoryUiDirty>,
                ),
            )
            .add_systems(
                OnExit(GameplayModalState::CharacterInfo),
                (
                    reset_resource::<InventoryCursor>,
                    reset_resource::<PlayerInventoryView>,
                ),
            )
            .add_systems(
                Update,
                (
                    remember_creative_scroll_positions,
                    handle_search_focus,
                    focus_inventory_search_frame,
                    handle_player_search_focus,
                    handle_inventory_close_shortcut.run_if(in_state(GameplayModalState::Inventory)),
                    handle_search_input,
                    handle_player_search_input,
                    handle_category_clicks,
                    handle_creative_scroll,
                    handle_creative_slot_clicks,
                    handle_slot_clicks,
                    handle_inventory_sort_clicks,
                    handle_inventory_trash_clicks,
                    handle_empty_inventory_click,
                    sync_search_focus,
                    sync_player_search_focus,
                )
                    .chain()
                    .in_set(InventoryHudSet::Input)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(inventory_ui_open),
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
                    .run_if(inventory_ui_open),
            )
            .add_systems(
                Update,
                (
                    style_search_bar,
                    frame_inventory_search_field,
                    style_inventory_search_field,
                    style_player_inventory_search_field,
                    style_category_buttons,
                    style_creative_slots,
                    style_inventory_slots,
                    style_inventory_trash_button,
                    sync_inventory_sort_tooltip,
                    update_cursor_icon_position,
                    sync_inventory_item_tooltip,
                )
                    .chain()
                    .in_set(InventoryHudSet::Style)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(inventory_ui_open),
            );
    }
}

fn inventory_ui_open(modal: Res<State<GameplayModalState>>) -> bool {
    modal.get().shows_inventory()
}
