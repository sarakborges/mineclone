use bevy::prelude::*;

use crate::{
    hud::{item_icon::spawn_hud_item_icon, item_stack_count::spawn_item_stack_count},
    ui::{surface, typography},
};

use super::{
    super::state::{
        ITEM_ICON_SIZE, InventoryCursorIcon, InventoryItemTooltip, InventoryItemTooltipHint,
        InventoryItemTooltipId, InventoryItemTooltipStats, InventoryItemTooltipStatsTitle,
        InventoryItemTooltipText,
    },
    InventoryItemView,
};

pub(in crate::hud::inventory) fn spawn_item_tooltip(root: &mut ChildSpawnerCommands) {
    root.spawn((
        InventoryItemTooltip,
        surface::hud_container(Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            max_width: px(360),
            padding: UiRect::axes(px(10), px(7)),
            border: UiRect::all(px(1)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: px(2),
            ..default()
        }),
        Visibility::Hidden,
        GlobalZIndex(200),
        Pickable::IGNORE,
    ))
    .with_children(|tooltip| {
        tooltip.spawn((
            InventoryItemTooltipText,
            typography::inventory_category(""),
            Node {
                max_width: px(338),
                ..default()
            },
            Pickable::IGNORE,
        ));
        tooltip.spawn((
            InventoryItemTooltipId,
            typography::caption(""),
            Node {
                max_width: px(338),
                ..default()
            },
            Pickable::IGNORE,
        ));
        tooltip.spawn((
            InventoryItemTooltipHint,
            typography::caption(""),
            TextLayout::no_wrap(),
            Node {
                max_width: px(338),
                margin: UiRect::top(px(6)),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
        tooltip.spawn((
            InventoryItemTooltipStatsTitle,
            typography::inventory_category(""),
            Node {
                max_width: px(338),
                margin: UiRect::top(px(6)),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
        tooltip.spawn((
            InventoryItemTooltipStats,
            typography::caption(""),
            Node {
                max_width: px(338),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
    });
}

pub(in crate::hud::inventory) fn spawn_cursor_icon(
    root: &mut ChildSpawnerCommands,
    item_id: &'static str,
    position: Vec2,
    items: &mut InventoryItemView<'_>,
) {
    root.spawn((
        InventoryCursorIcon,
        Node {
            position_type: PositionType::Absolute,
            left: px(position.x - ITEM_ICON_SIZE * 0.5),
            top: px(position.y - ITEM_ICON_SIZE * 0.5),
            width: px(ITEM_ICON_SIZE),
            height: px(ITEM_ICON_SIZE),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|cursor| {
        spawn_hud_item_icon(cursor, item_id, items, ITEM_ICON_SIZE, None, None);
    });
}

pub(in crate::hud::inventory) fn spawn_cursor_stack_count(
    root: &mut ChildSpawnerCommands,
    quantity: u32,
    position: Vec2,
) {
    if quantity <= 1 {
        return;
    }

    root.spawn((
        InventoryCursorIcon,
        Node {
            position_type: PositionType::Absolute,
            left: px(position.x - ITEM_ICON_SIZE * 0.5),
            top: px(position.y - ITEM_ICON_SIZE * 0.5),
            width: px(ITEM_ICON_SIZE),
            height: px(ITEM_ICON_SIZE),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|overlay| {
        spawn_item_stack_count(overlay, quantity);
    });
}

pub(in crate::hud::inventory) fn spawn_inventory_item(
    slot: &mut ChildSpawnerCommands,
    item_id: &'static str,
    items: &mut InventoryItemView<'_>,
) {
    spawn_hud_item_icon(slot, item_id, items, ITEM_ICON_SIZE, None, None);
}
