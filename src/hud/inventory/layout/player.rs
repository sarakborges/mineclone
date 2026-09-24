use bevy::{
    prelude::*,
    text::{EditableText, FontWeight},
};

use crate::{
    content::builtin_ids::BIOME_TINT_METADATA_KEY,
    hud::item_stack_count::spawn_item_stack_count,
    player::hotbar::{HOTBAR_INVENTORY_OFFSET, HOTBAR_SLOT_COUNT, PlayerHotbar},
    ui::{
        button::{self, ButtonVariant},
        selectable, surface, text_input, theme, typography,
    },
};

use super::{
    InventoryItemView, InventoryLayoutState,
    item::spawn_inventory_item,
    super::state::{
        InventorySearchBar, InventorySearchFrame, InventorySearchText, InventorySlot,
        InventorySortButton, InventorySortTooltip, InventoryTrashButton, PANEL_BORDER_WIDTH,
        PANEL_PADDING, PLAYER_HEADER_GAP, PLAYER_SEARCH_WIDTH, SEARCH_HEIGHT, SECTION_GAP,
        SLOT_GAP, SLOT_SIZE, TRASH_GAP,
    },
};

pub(super) fn spawn_player_inventory_panel(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    let hotbar = state.hotbar;

    root.spawn(surface::hud_container(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexStart,
        row_gap: px(SECTION_GAP),
        padding: UiRect::all(px(PANEL_PADDING)),
        border: UiRect::all(px(PANEL_BORDER_WIDTH)),
        ..default()
    }))
    .insert(Pickable::IGNORE)
    .with_children(|panel| {
        panel
            .spawn((
                Node {
                    width: px(player_panel_content_width()),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(PLAYER_HEADER_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|header| {
                header.spawn((typography::hud_heading("Inventory"), Pickable::IGNORE));

                header
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(PLAYER_HEADER_GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|controls| {
                        spawn_inventory_search_field(controls, state, items);
                        spawn_inventory_sort_button(controls, state, items);
                    });
            });

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

        spawn_player_hotbar_footer(panel, hotbar, items);
    });
}

fn spawn_inventory_search_field(
    parent: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &InventoryItemView<'_>,
) {
    let placeholder_visible =
        state.player_view.search_query().is_empty() && !state.player_view.search_focused();
    let placeholder = state
        .localization
        .text(items.language, "inventory.searchPlaceholder");

    parent
        .spawn((
            Button,
            InventorySearchFrame,
            Node {
                position_type: PositionType::Relative,
                width: px(PLAYER_SEARCH_WIDTH),
                height: px(SEARCH_HEIGHT),
                padding: UiRect::horizontal(px(text_input::INPUT_PADDING_X)),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            text_input::frame_surface(state.player_view.search_focused()),
        ))
        .with_children(|frame| {
            frame.spawn((
                InventorySearchBar,
                EditableText {
                    max_characters: Some(128),
                    ..EditableText::new(state.player_view.search_query())
                },
                text_input::editor_style(17.0, FontWeight::NORMAL),
                Node {
                    width: percent(100),
                    min_width: px(0),
                    height: px(text_input::INPUT_EDITOR_HEIGHT),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            frame.spawn((
                InventorySearchText,
                typography::hud(placeholder),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(text_input::INPUT_PADDING_X + 1.0),
                    top: px(text_input::centered_text_top(SEARCH_HEIGHT)),
                    ..default()
                },
                if placeholder_visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                Pickable::IGNORE,
            ));
        });
}

fn spawn_inventory_sort_button(
    parent: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &InventoryItemView<'_>,
) {
    let tooltip = state
        .localization
        .text(items.language, "inventory.sortBackpack");

    parent
        .spawn(button::icon_button(
            InventorySortButton,
            SEARCH_HEIGHT,
            ButtonVariant::Normal,
        ))
        .with_children(|button| {
            button.spawn((
                typography::button_label_light("⇅"),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ));
            button
                .spawn((
                    InventorySortTooltip,
                    surface::hud_container(Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        bottom: px(SEARCH_HEIGHT + 8.0),
                        padding: UiRect::axes(px(9), px(6)),
                        border: UiRect::all(px(1)),
                        ..default()
                    }),
                    Visibility::Hidden,
                    GlobalZIndex(210),
                    Pickable::IGNORE,
                ))
                .with_children(|hint| {
                    hint.spawn((
                        typography::caption(tooltip),
                        TextLayout::no_wrap(),
                        Pickable::IGNORE,
                    ));
                });
        });
}

pub(super) fn spawn_player_hotbar_footer(
    parent: &mut ChildSpawnerCommands,
    hotbar: &PlayerHotbar,
    items: &mut InventoryItemView<'_>,
) {
    parent
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
                            false,
                            hotbar,
                            items,
                        );
                    }
                });

            spawn_inventory_trash_button(footer);
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
    let stack = hotbar.inventory_stack_at(index);
    let item = stack.map(crate::player::item_stack::ItemStack::id);
    let quantity = stack.map_or(0, crate::player::item_stack::ItemStack::quantity);

    parent
        .spawn((
            Button,
            InventorySlot {
                index,
                item,
                quantity,
            },
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
                let biome_override = stack.and_then(|stack| stack.metadata().get(BIOME_TINT_METADATA_KEY));
                spawn_inventory_item(slot, item_id, biome_override, items);
                spawn_item_stack_count(slot, quantity);
            }
        });
}

pub(super) fn player_panel_content_width() -> f32 {
    HOTBAR_SLOT_COUNT as f32 * SLOT_SIZE
        + (HOTBAR_SLOT_COUNT - 1) as f32 * SLOT_GAP
        + TRASH_GAP
        + SLOT_SIZE
}
