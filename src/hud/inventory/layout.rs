mod creative;
mod item;
mod player;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::inventory_category::InventoryCategoryRegistry,
    gameplay::modal::GameplayModalState,
    hud::character_info::spawn_character_info_panel,
    localization::UiLocalization,
    player::{
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        inventory::InventoryCursor,
    },
    ui::button::{self, ButtonVariant},
};

pub(super) use crate::hud::item_icon::HudItemIconView as InventoryItemView;

pub(super) use self::{
    creative::spawn_creative_catalog_rows,
    item::{
        spawn_cursor_icon, spawn_cursor_stack_count, spawn_inventory_item, spawn_item_tooltip,
    },
};
use self::{creative::spawn_creative_panel, player::spawn_player_inventory_panel};
use super::state::{
    InventoryHudRoot, InventoryViewPane, InventoryViewToggleButton, PANEL_GAP,
};

const INVENTORY_VIEW_TOGGLE_WIDTH: f32 = 96.0;
const INVENTORY_VIEW_TOGGLE_HEIGHT: f32 = 40.0;
const INVENTORY_VIEW_TOGGLE_GAP: f32 = 8.0;
const INVENTORY_VIEW_TOGGLE_TOP_MARGIN: f32 = 12.0;
const INVENTORY_VIEW_TOGGLE_BORDER_WIDTH: f32 = 2.0;
const SURVIVAL_CRAFTING_HEIGHT: f32 = 440.0;
const SURVIVAL_STATION_WIDTH: f32 = 244.0;

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
    spawn_inventory_overlay(root, state, items);
}

fn spawn_game_mode_inventory_panel(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    if state.game_mode.has_creative_inventory() {
        spawn_inventory_switcher(root, state, items, true);
    } else {
        spawn_player_inventory_panel(root, state, items);
    }
}

fn spawn_survival_inventory_row(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    root.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexStart,
            column_gap: px(PANEL_GAP),
            margin: UiRect::top(px(PANEL_GAP)),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|row| {
        spawn_character_info_panel(row);

        row.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                margin: UiRect::top(px(SURVIVAL_CRAFTING_HEIGHT + PANEL_GAP)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|main_column| {
            spawn_player_inventory_panel(main_column, state, items);
        });

        row.spawn((
            Node {
                width: px(SURVIVAL_STATION_WIDTH),
                min_width: px(SURVIVAL_STATION_WIDTH),
                ..default()
            },
            Pickable::IGNORE,
        ));
    });
}

fn spawn_inventory_switcher(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
    creative_visible: bool,
) {
    root.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexStart,
            column_gap: px(0),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|switcher| {
        switcher
            .spawn((
                InventoryViewPane { creative: false },
                Node {
                    display: if creative_visible {
                        Display::None
                    } else {
                        Display::Flex
                    },
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|pane| {
                spawn_player_inventory_panel(pane, state, items);
            });

        switcher
            .spawn((
                InventoryViewPane { creative: true },
                Node {
                    display: if creative_visible {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|pane| {
                spawn_creative_panel(pane, state, items);
            });

        switcher
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(INVENTORY_VIEW_TOGGLE_GAP),
                    margin: UiRect::top(px(INVENTORY_VIEW_TOGGLE_TOP_MARGIN)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|tabs| {
                spawn_inventory_view_toggle_button(tabs, "Inventory", false, !creative_visible);
                spawn_inventory_view_toggle_button(tabs, "Creative", true, creative_visible);
            });
    });
}

fn spawn_inventory_view_toggle_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    creative: bool,
    active: bool,
) {
    parent
        .spawn(button::button(
            label,
            InventoryViewToggleButton { creative },
            px(INVENTORY_VIEW_TOGGLE_WIDTH),
            INVENTORY_VIEW_TOGGLE_HEIGHT,
            ButtonVariant::from_active(active),
        ))
        .insert(Node {
            width: px(INVENTORY_VIEW_TOGGLE_WIDTH),
            height: px(INVENTORY_VIEW_TOGGLE_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect {
                left: px(0),
                right: px(INVENTORY_VIEW_TOGGLE_BORDER_WIDTH),
                top: px(INVENTORY_VIEW_TOGGLE_BORDER_WIDTH),
                bottom: px(INVENTORY_VIEW_TOGGLE_BORDER_WIDTH),
            },
            ..default()
        });
}

fn spawn_inventory_overlay(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
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

pub(super) fn spawn_inventory_root(
    commands: &mut Commands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    let creative_inventory = state.game_mode.has_creative_inventory();

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
                align_items: if creative_inventory {
                    AlignItems::Center
                } else {
                    AlignItems::FlexStart
                },
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
            if creative_inventory {
                spawn_game_mode_inventory_panel(root, state, items);
            } else {
                spawn_survival_inventory_row(root, state, items);
            }
            spawn_inventory_overlay(root, state, items);
        });
}
