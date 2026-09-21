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
        layer::LayerRegistry,
        tool::ToolRegistry,
    },
    localization::{Language, UiLocalization},
    player::{
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        inventory::{InventoryCursor, InventoryState},
    },
    world::biome_field::BiomeField,
};

use crate::hud::block_icon::BlockIconMaterial;

pub(super) use self::{
    creative::spawn_creative_catalog_rows,
    item::{spawn_cursor_icon, spawn_inventory_item, spawn_item_tooltip},
};
use self::{creative::spawn_creative_panel, player::spawn_player_inventory_panel};
use super::state::{InventoryHudRoot, PANEL_GAP};

pub(super) struct InventoryItemView<'a> {
    pub(super) asset_server: &'a AssetServer,
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
    pub(super) scroll_state: &'a super::state::CreativeScrollState,
    pub(super) localization: &'a UiLocalization,
    pub(super) game_mode: GameMode,
    pub(super) cursor_position: Option<Vec2>,
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
            DespawnOnExit(InventoryState::Open),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            if state.game_mode.has_creative_inventory() {
                spawn_creative_panel(root, state, items);
            }
            spawn_player_inventory_panel(root, state.hotbar, items);
            spawn_item_tooltip(root);

            let Some(item_id) = state.cursor.item() else {
                return;
            };
            spawn_cursor_icon(
                root,
                item_id,
                state.cursor_position.unwrap_or(Vec2::ZERO),
                items,
            );
        });
}
