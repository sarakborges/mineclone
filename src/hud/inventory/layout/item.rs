use bevy::prelude::*;

use crate::{
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    ui::{surface, typography},
};

use crate::hud::{layer_icon::spawn_layer_icon, tool_icon::spawn_tool_icon};

use super::{
    InventoryItemView,
    super::state::{
        InventoryCursorIcon, InventoryItemTooltip, InventoryItemTooltipText, ITEM_ICON_SIZE,
    },
};

pub(in crate::hud::inventory) fn spawn_item_tooltip(root: &mut ChildSpawnerCommands) {
    root.spawn((
        InventoryItemTooltip,
        surface::hud_container(Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            max_width: px(280),
            padding: UiRect::axes(px(10), px(7)),
            border: UiRect::all(px(1)),
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
    if let Some(block) = items.blocks.get(item_id) {
        let tint = block_tint_at(
            block.tint,
            items.player_position,
            items.biome_field,
            items.biomes,
        );
        let material = items
            .icon_materials
            .add(crate::hud::block_icon::BlockIconMaterial::from_block(
                block,
                items.asset_server,
                tint,
            ));

        root.spawn((
            InventoryCursorIcon,
            BlockModel::display(item_id),
            MaterialNode(material),
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x - ITEM_ICON_SIZE * 0.5),
                top: px(position.y - ITEM_ICON_SIZE * 0.5),
                width: px(ITEM_ICON_SIZE),
                height: px(ITEM_ICON_SIZE),
                ..default()
            },
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(layer) = items.layers.get(item_id) {
        let tint = block_tint_at(
            layer.tint,
            items.player_position,
            items.biome_field,
            items.biomes,
        );
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
        .with_children(|cursor| {
            spawn_layer_icon(
                cursor,
                layer,
                items.asset_server,
                tint,
                ITEM_ICON_SIZE,
            );
        });
        return;
    }

    if let Some(tool) = items.tools.get(item_id) {
        if tool.icon.is_empty() {
            root.spawn((
                InventoryCursorIcon,
                typography::caption(tool.name.text(items.language)),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(position.x - 36.0),
                    top: px(position.y - ITEM_ICON_SIZE * 0.5),
                    width: px(72),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        } else {
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
            .with_children(|cursor| {
                spawn_tool_icon(
                    cursor,
                    tool,
                    items.asset_server,
                    items.brush_mode,
                    items.dyes,
                    items.language,
                    ITEM_ICON_SIZE,
                );
            });
        }
        return;
    }

    panic!("inventory cursor references missing item: {item_id}");
}

pub(in crate::hud::inventory) fn spawn_inventory_item(
    slot: &mut ChildSpawnerCommands,
    item_id: &'static str,
    items: &mut InventoryItemView<'_>,
) {
    if let Some(block) = items.blocks.get(item_id) {
        let tint = block_tint_at(
            block.tint,
            items.player_position,
            items.biome_field,
            items.biomes,
        );
        let material = items
            .icon_materials
            .add(crate::hud::block_icon::BlockIconMaterial::from_block(
                block,
                items.asset_server,
                tint,
            ));

        slot.spawn((
            BlockModel::display(item_id),
            MaterialNode(material),
            Node {
                width: px(ITEM_ICON_SIZE),
                height: px(ITEM_ICON_SIZE),
                ..default()
            },
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(layer) = items.layers.get(item_id) {
        let tint = block_tint_at(
            layer.tint,
            items.player_position,
            items.biome_field,
            items.biomes,
        );
        spawn_layer_icon(
            slot,
            layer,
            items.asset_server,
            tint,
            ITEM_ICON_SIZE,
        );
        return;
    }

    if let Some(tool) = items.tools.get(item_id) {
        spawn_tool_icon(
            slot,
            tool,
            items.asset_server,
            items.brush_mode,
            items.dyes,
            items.language,
            ITEM_ICON_SIZE,
        );
        return;
    }

    panic!("inventory references missing item: {item_id}");
}
