use bevy::prelude::*;

use crate::{
    player::hotbar::{HOTBAR_INVENTORY_OFFSET, HOTBAR_SLOT_COUNT, PlayerHotbar},
    ui::{selectable, surface, theme, typography},
};

use super::{
    InventoryItemView,
    item::spawn_inventory_item,
    super::state::{
        InventorySlot, InventoryTrashButton, PANEL_PADDING, SECTION_GAP, SLOT_GAP, SLOT_SIZE,
        TRASH_GAP,
    },
};

pub(super) fn spawn_player_inventory_panel(
    root: &mut ChildSpawnerCommands,
    hotbar: &PlayerHotbar,
    items: &mut InventoryItemView<'_>,
) {
    root.spawn(surface::hud_container(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexStart,
        row_gap: px(SECTION_GAP),
        padding: UiRect::all(px(PANEL_PADDING)),
        border: UiRect::all(px(1)),
        ..default()
    }))
    .insert(Pickable::IGNORE)
    .with_children(|panel| {
        panel.spawn((typography::hud_subheading("Inventory"), Pickable::IGNORE));

        panel
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(SLOT_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|backpack| {
                for row in 0..3 {
                    backpack
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: px(SLOT_GAP),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|row_node| {
                            for column in 0..HOTBAR_SLOT_COUNT {
                                let index = row * HOTBAR_SLOT_COUNT + column;
                                spawn_slot(row_node, index, false, hotbar, items);
                            }
                        });
                }
            });

        panel
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(TRASH_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|footer| {
                footer
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(SLOT_GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|hotbar_row| {
                        for hotbar_index in 0..HOTBAR_SLOT_COUNT {
                            spawn_slot(
                                hotbar_row,
                                HOTBAR_INVENTORY_OFFSET + hotbar_index,
                                hotbar_index == hotbar.selected_slot(),
                                hotbar,
                                items,
                            );
                        }
                    });

                spawn_inventory_trash_button(footer);
            });
    });
}

fn spawn_inventory_trash_button(parent: &mut ChildSpawnerCommands) {
    let (background, border) = selectable::danger_colors(Interaction::None);
    let icon_color = theme::TEXT_PRIMARY;

    parent
        .spawn((
            Button,
            InventoryTrashButton,
            Node {
                width: px(SLOT_SIZE),
                height: px(SLOT_SIZE),
                min_width: px(SLOT_SIZE),
                min_height: px(SLOT_SIZE),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(border),
        ))
        .with_children(|button| {
            button
                .spawn((
                    Node {
                        width: px(22),
                        height: px(25),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(2),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|icon| {
                    icon.spawn((
                        Node {
                            width: px(8),
                            height: px(3),
                            ..default()
                        },
                        BackgroundColor(icon_color),
                        Pickable::IGNORE,
                    ));
                    icon.spawn((
                        Node {
                            width: px(20),
                            height: px(3),
                            ..default()
                        },
                        BackgroundColor(icon_color),
                        Pickable::IGNORE,
                    ));
                    icon.spawn((
                        Node {
                            width: px(16),
                            height: px(16),
                            border: UiRect::all(px(2)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(icon_color),
                        Pickable::IGNORE,
                    ));
                });
        });
}

fn spawn_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    selected: bool,
    hotbar: &PlayerHotbar,
    items: &mut InventoryItemView<'_>,
) {
    let (background, border) = selectable::static_colors(selected);
    let item = hotbar.inventory_item_at(index);

    parent
        .spawn((
            Button,
            InventorySlot { index, item },
            Node {
                width: px(SLOT_SIZE),
                height: px(SLOT_SIZE),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(border),
        ))
        .with_children(|slot| {
            if let Some(item_id) = item {
                spawn_inventory_item(slot, item_id, items);
            }
        });
}
