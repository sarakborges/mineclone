mod creative;
mod item;
mod player;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        inventory_category::InventoryCategoryRegistry,
        item::ItemRegistry,
        layer::LayerRegistry,
        tool::ToolRegistry,
    },
    gameplay::modal::GameplayModalState,
    localization::{Language, UiLocalization},
    player::{
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        inventory::InventoryCursor,
    },
    world::biome_field::BiomeField,
};

use crate::hud::block_icon::BlockIconMaterial;

pub(super) use self::{
    creative::spawn_creative_catalog_rows,
    item::{
        spawn_cursor_icon, spawn_cursor_stack_count, spawn_inventory_item, spawn_item_tooltip,
    },
};
use self::{creative::spawn_creative_panel, player::spawn_player_inventory_panel};
use super::state::{InventoryHudRoot, PANEL_GAP};

pub(super) struct InventoryItemView<'a> {
    pub(super) asset_server: &'a AssetServer,
    pub(super) items: &'a ItemRegistry,
    pub(super) blocks: &'a BlockRegistry,
    pub(super) layers: &'a LayerRegistry,
    pub(super) tools: &'a ToolRegistry,
    pub(super) dyes: &'a crate::content::secondary_property::SecondaryPropertyRegistry,
    pub(super) brush_mode: &'a crate::tools::BrushMode,
    pub(super) biomes: &'a BiomeRegistry,
    pub(super) biome_field: &'a BiomeField,
    pub(super) player_position: Vec2,
    pub(super) language: Language,
    pub(super) icon_materials: &'a mut Assets<BlockIconMaterial>,
}

pub(super) struct InventoryLayoutState<'a> {
    pub(super) categories: &'a InventoryCategoryRegistry,
    pub(super) hotbar: &'a PlayerHotbar,
    pub(super) cursor: &'a InventoryCursor,
    pub(super) creative_view: &'a super::state::CreativeInventoryView,
    pub(super) player_view: &'a super::state::PlayerInventoryView,
    pub(super) scroll_state: &'a super::state::CreativeScrollState,
    pub(super) localization: &'a UiLocalization,
    pub(super) game_mode: GameMode,
    pub(super) cursor_position: Option<Vec2>,
}

pub(super) fn spawn_character_info_inventory(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    spawn_game_mode_inventory_panel(root, state, items);
    spawn_item_tooltip(root);

    let Some(item_id) = state.cursor.item() else {
        return;
    };
    let position = state.cursor_position.unwrap_or(Vec2::ZERO);
    spawn_cursor_icon(root, item_id, position, items);
    spawn_cursor_stack_count(
        root,
        state
            .cursor
            .stack()
            .map_or(0, crate::player::item_stack::ItemStack::quantity),
        position,
    );
}

fn spawn_game_mode_inventory_panel(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    if state.game_mode.has_creative_inventory() {
        spawn_creative_panel(root, state, items);
    } else {
        spawn_player_inventory_panel(root, state, items);
    }
}

pub(super) fn spawn_inventory_root(
    commands: &mut Commands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    commands
        .spawn((
            InventoryHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(PANEL_GAP),
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
            DespawnOnExit(GameplayModalState::Inventory),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            spawn_game_mode_inventory_panel(root, state, items);
            spawn_item_tooltip(root);

            let Some(item_id) = state.cursor.item() else {
                return;
            };
            let position = state.cursor_position.unwrap_or(Vec2::ZERO);
            spawn_cursor_icon(root, item_id, position, items);
            spawn_cursor_stack_count(
                root,
                state
                    .cursor
                    .stack()
                    .map_or(0, crate::player::item_stack::ItemStack::quantity),
                position,
            );
        });
}
