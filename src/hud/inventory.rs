mod interaction;
mod layout;
mod search_style;
mod state;
mod sync;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        inventory_category::InventoryCategoryRegistry,
        layer::LayerRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::{
        camera::GameplayCamera,
        character_info::CharacterInfoState,
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        inventory::{InventoryCursor, InventoryState},
    },
    tools::BrushMode,
    world::biome_field::BiomeField,
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
use layout::{InventoryItemView, InventoryLayoutState, spawn_character_info_inventory};
use state::{
    CreativeInventoryUiDirty, CreativeInventoryView, CreativeScrollState, PlayerInventoryView,
};
use sync::{
    rebuild_inventory_when_changed, spawn_inventory, style_category_buttons, style_creative_slots,
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

#[derive(SystemParam)]
pub(super) struct CharacterInfoInventorySpawn<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    layers: Res<'w, LayerRegistry>,
    tools: Res<'w, ToolRegistry>,
    dyes: Res<'w, SecondaryPropertyRegistry>,
    brush_mode: Res<'w, BrushMode>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    language: Res<'w, ActiveLanguage>,
    categories: Res<'w, InventoryCategoryRegistry>,
    localization: Res<'w, UiLocalization>,
    hotbar: Res<'w, PlayerHotbar>,
    cursor: Res<'w, InventoryCursor>,
    creative_view: Res<'w, CreativeInventoryView>,
    player_view: Res<'w, PlayerInventoryView>,
    scroll_state: Res<'w, CreativeScrollState>,
    player: Single<'w, 's, (&'static Transform, &'static GameMode), With<GameplayCamera>>,
    window: Single<'w, 's, &'static Window>,
    icon_materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

impl CharacterInfoInventorySpawn<'_, '_> {
    pub(super) fn spawn(&mut self, root: &mut ChildSpawnerCommands) {
        let (player_transform, game_mode) = *self.player;
        let player_position = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.z,
        );
        let mut items = InventoryItemView {
            asset_server: &self.asset_server,
            blocks: &self.blocks,
            layers: &self.layers,
            tools: &self.tools,
            dyes: &self.dyes,
            brush_mode: &self.brush_mode,
            biomes: &self.biomes,
            biome_field: &self.biome_field,
            player_position,
            language: self.language.get(),
            icon_materials: &mut self.icon_materials,
        };
        let layout = InventoryLayoutState {
            categories: &self.categories,
            hotbar: &self.hotbar,
            cursor: &self.cursor,
            creative_view: &self.creative_view,
            player_view: &self.player_view,
            scroll_state: &self.scroll_state,
            localization: &self.localization,
            game_mode: *game_mode,
            cursor_position: self.window.cursor_position(),
        };

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
                OnEnter(InventoryState::Open),
                spawn_inventory.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(InventoryState::Open),
                (
                    reset_resource::<CreativeInventoryView>,
                    reset_resource::<PlayerInventoryView>,
                    reset_resource::<CreativeScrollState>,
                    reset_resource::<CreativeInventoryUiDirty>,
                ),
            )
            .add_systems(
                OnExit(CharacterInfoState::Open),
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
                    handle_inventory_close_shortcut.run_if(in_state(InventoryState::Open)),
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

fn inventory_ui_open(
    inventory: Res<State<InventoryState>>,
    character_info: Res<State<CharacterInfoState>>,
) -> bool {
    *inventory.get() == InventoryState::Open
        || *character_info.get() == CharacterInfoState::Open
}
