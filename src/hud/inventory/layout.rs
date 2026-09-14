use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry,
        block::{BlockDefinition, BlockRegistry},
        block_id::intern_block_id,
        inventory_category::{InventoryCategoryDefinition, InventoryCategoryRegistry},
        tool::{ToolDefinition, ToolRegistry},
        tool_id::intern_tool_id,
    },
    localization::{Language, UiLocalization},
    player::{
        game_mode::GameMode,
        hotbar::{HOTBAR_INVENTORY_OFFSET, HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::{InventoryCursor, InventoryState},
    },
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    ui::{scrollbar, surface, theme, typography},
    world::biome_field::BiomeField,
};

use crate::hud::block_icon::BlockIconMaterial;

use super::state::{
    CATEGORY_GAP, CATEGORY_ICON_SIZE, CATEGORY_ROW_HEIGHT, CATEGORY_WIDTH, CREATIVE_COLUMNS,
    CREATIVE_GRID_HEIGHT, CreativeCatalogScrollArea, CreativeCatalogScrollbar,
    CreativeCategoryButton, CreativeCategoryScrollArea, CreativeCategoryScrollbar,
    CreativeInventorySlot, CreativeInventoryView, CreativeScrollState, CreativeSearchBar,
    CreativeSearchText, ITEM_ICON_SIZE, InventoryCursorIcon, InventoryHudRoot, InventorySlot,
    InventoryTrashButton, PANEL_GAP, PANEL_PADDING, SCROLLBAR_TOTAL_WIDTH, SEARCH_GAP,
    SEARCH_HEIGHT, SECTION_GAP, SLOT_GAP, SLOT_SIZE, TRASH_GAP,
};

#[derive(Clone, Copy)]
enum CreativeCatalogItem<'a> {
    Block(&'a BlockDefinition),
    Tool(&'a ToolDefinition),
}

impl<'a> CreativeCatalogItem<'a> {
    fn id(self) -> &'a str {
        match self {
            Self::Block(block) => &block.id,
            Self::Tool(tool) => &tool.id,
        }
    }

    fn category(self) -> &'a str {
        match self {
            Self::Block(block) => &block.category,
            Self::Tool(tool) => &tool.category,
        }
    }

    fn name(self, language: Language) -> &'a str {
        match self {
            Self::Block(block) => block.name.text(language),
            Self::Tool(tool) => tool.name.text(language),
        }
    }

    fn interned_id(self) -> &'static str {
        match self {
            Self::Block(block) => intern_block_id(&block.id),
            Self::Tool(tool) => intern_tool_id(&tool.id),
        }
    }
}

pub(super) struct InventoryItemView<'a> {
    pub(super) asset_server: &'a AssetServer,
    pub(super) blocks: &'a BlockRegistry,
    pub(super) tools: &'a ToolRegistry,
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
    pub(super) creative_view: &'a CreativeInventoryView,
    pub(super) scroll_state: &'a CreativeScrollState,
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

pub(super) fn spawn_cursor_icon(
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
        let material = items.icon_materials.add(BlockIconMaterial::from_block(
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

    if let Some(tool) = items.tools.get(item_id) {
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
        return;
    }

    panic!("inventory cursor references missing item: {item_id}");
}

fn spawn_creative_panel(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    root.spawn(surface::hud_container(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(SEARCH_GAP),
        padding: UiRect::all(px(PANEL_PADDING)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(8)),
        ..default()
    }))
    .insert(Pickable::IGNORE)
    .with_children(|panel| {
        spawn_search_bar(
            panel,
            state.creative_view,
            state.localization,
            items.language,
        );

        panel
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    column_gap: px(CATEGORY_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|content| {
                spawn_category_list(
                    content,
                    state.categories,
                    state.creative_view,
                    state.scroll_state.category_y,
                    state.localization,
                    items,
                );

                content
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Stretch,
                            height: px(CREATIVE_GRID_HEIGHT),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|catalog_content| {
                        let scroll_area = catalog_content
                            .spawn((
                                CreativeCatalogScrollArea,
                                ScrollPosition(Vec2::new(0.0, state.scroll_state.catalog_y)),
                                Node {
                                    width: px(creative_grid_width()),
                                    height: px(CREATIVE_GRID_HEIGHT),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(SLOT_GAP),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|scroll| {
                                spawn_creative_catalog_rows(
                                    scroll,
                                    state.categories,
                                    state.creative_view,
                                    state.cursor.item(),
                                    items,
                                );
                            })
                            .id();

                        catalog_content
                            .spawn(scrollbar::vertical_scrollbar(scroll_area))
                            .insert(CreativeCatalogScrollbar);
                    });
            });
    });
}

fn spawn_search_bar(
    parent: &mut ChildSpawnerCommands,
    creative_view: &CreativeInventoryView,
    localization: &UiLocalization,
    language: Language,
) {
    let search_border = if creative_view.search_focused() {
        surface::HUD_SELECTED_BORDER_COLOR
    } else {
        surface::HUD_BORDER_COLOR
    };
    let search_text = if creative_view.search_query().is_empty() {
        localization.text(language, "inventory.searchPlaceholder")
    } else {
        creative_view.search_query()
    };

    parent
        .spawn((
            Button,
            CreativeSearchBar,
            Node {
                width: px(creative_content_width()),
                height: px(SEARCH_HEIGHT),
                padding: UiRect::horizontal(px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme::HUD_SURFACE),
            BorderColor::all(search_border),
        ))
        .with_children(|search| {
            search.spawn((
                CreativeSearchText,
                typography::hud(search_text),
                Pickable::IGNORE,
            ));
        });
}

fn spawn_category_list(
    parent: &mut ChildSpawnerCommands,
    categories: &InventoryCategoryRegistry,
    creative_view: &CreativeInventoryView,
    initial_scroll_y: f32,
    localization: &UiLocalization,
    items: &mut InventoryItemView<'_>,
) {
    let mut ordered = categories.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.id.cmp(&right.id))
    });

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                height: px(CREATIVE_GRID_HEIGHT),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|category_content| {
            let scroll_area = category_content
                .spawn((
                    CreativeCategoryScrollArea,
                    ScrollPosition(Vec2::new(0.0, initial_scroll_y)),
                    Node {
                        width: px(CATEGORY_WIDTH),
                        height: px(CREATIVE_GRID_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(SLOT_GAP),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|list| {
                    spawn_category_button(
                        list,
                        None,
                        creative_view.selected_category().is_none(),
                        localization,
                        items,
                    );

                    for category in ordered {
                        let selected =
                            creative_view.selected_category() == Some(category.id.as_str());
                        spawn_category_button(list, Some(category), selected, localization, items);
                    }
                })
                .id();

            category_content
                .spawn(scrollbar::vertical_scrollbar(scroll_area))
                .insert(CreativeCategoryScrollbar);
        });
}

fn spawn_category_button(
    parent: &mut ChildSpawnerCommands,
    category: Option<&InventoryCategoryDefinition>,
    selected: bool,
    localization: &UiLocalization,
    items: &mut InventoryItemView<'_>,
) {
    let id = category.map(|category| category.id.clone());
    let label = match category {
        Some(category) => category.display_name.text(items.language),
        None => localization.text(items.language, "inventory.everything"),
    };
    let (background, border) = surface::hud_control_static(selected);

    parent
        .spawn((
            Button,
            CreativeCategoryButton { id },
            Node {
                width: percent(100),
                height: px(CATEGORY_ROW_HEIGHT),
                min_height: px(CATEGORY_ROW_HEIGHT),
                padding: UiRect::horizontal(px(8)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(4)),
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(border),
        ))
        .with_children(|button| {
            if let Some((category, block_icon)) = category
                .and_then(|category| category.block_icon.as_ref().map(|icon| (category, icon)))
            {
                let block = items.blocks.get(&block_icon.block).unwrap_or_else(|| {
                    panic!(
                        "inventory category {} references missing block icon {}",
                        category.id, block_icon.block
                    )
                });
                let block_id = intern_block_id(&block.id);
                let material = items.icon_materials.add(BlockIconMaterial::from_block(
                    block,
                    items.asset_server,
                    category.icon_tint(block),
                ));

                button.spawn((
                    BlockModel::display(block_id),
                    MaterialNode(material),
                    Node {
                        width: px(CATEGORY_ICON_SIZE),
                        height: px(CATEGORY_ICON_SIZE),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }

            button.spawn((typography::inventory_category(label), Pickable::IGNORE));
        });
}

pub(super) fn spawn_creative_catalog_rows(
    parent: &mut ChildSpawnerCommands,
    categories: &InventoryCategoryRegistry,
    creative_view: &CreativeInventoryView,
    selected_item: Option<&'static str>,
    items: &mut InventoryItemView<'_>,
) {
    let catalog = filtered_creative_catalog(
        items.blocks,
        items.tools,
        categories,
        creative_view.search_query(),
        creative_view.selected_category(),
        items.language,
    );
    spawn_creative_grid(parent, &catalog, selected_item, items);
}

fn spawn_creative_grid(
    parent: &mut ChildSpawnerCommands,
    catalog: &[CreativeCatalogItem<'_>],
    selected_item: Option<&'static str>,
    items: &mut InventoryItemView<'_>,
) {
    let total_rows = catalog.len().div_ceil(CREATIVE_COLUMNS).max(1);

    for row in 0..total_rows {
        parent
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(SLOT_GAP),
                    min_height: px(SLOT_SIZE),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row_node| {
                for column in 0..CREATIVE_COLUMNS {
                    let item = catalog.get(row * CREATIVE_COLUMNS + column).copied();
                    spawn_creative_slot(row_node, item, selected_item, items);
                }
            });
    }
}

fn spawn_player_inventory_panel(
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
        border_radius: BorderRadius::all(px(8)),
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
    let (background, border) = surface::hud_danger_control_colors(Interaction::None);
    let icon_color = Color::srgb(0.94, 0.40, 0.44);

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
                border_radius: BorderRadius::all(px(4)),
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
                            border_radius: BorderRadius::all(px(2)),
                            ..default()
                        },
                        BackgroundColor(icon_color),
                        Pickable::IGNORE,
                    ));
                    icon.spawn((
                        Node {
                            width: px(20),
                            height: px(3),
                            border_radius: BorderRadius::all(px(2)),
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
                            border_radius: BorderRadius::all(px(2)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(icon_color),
                        Pickable::IGNORE,
                    ));
                });
        });
}

fn spawn_creative_slot(
    parent: &mut ChildSpawnerCommands,
    item: Option<CreativeCatalogItem<'_>>,
    selected_item: Option<&'static str>,
    items: &mut InventoryItemView<'_>,
) {
    let item_id = item.map(CreativeCatalogItem::interned_id);
    let selected = item_id.is_some() && item_id == selected_item;
    let (background, border) = surface::hud_control_static(selected);

    parent
        .spawn((
            Button,
            CreativeInventorySlot { item: item_id },
            Node {
                width: px(SLOT_SIZE),
                height: px(SLOT_SIZE),
                min_width: px(SLOT_SIZE),
                min_height: px(SLOT_SIZE),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(4)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(border),
        ))
        .with_children(|slot| {
            let Some(item) = item else {
                return;
            };

            match item {
                CreativeCatalogItem::Block(block) => {
                    let block_id = intern_block_id(&block.id);
                    let tint = block_tint_at(
                        block.tint,
                        items.player_position,
                        items.biome_field,
                        items.biomes,
                    );
                    let material = items.icon_materials.add(BlockIconMaterial::from_block(
                        block,
                        items.asset_server,
                        tint,
                    ));

                    slot.spawn((
                        BlockModel::display(block_id),
                        MaterialNode(material),
                        Node {
                            width: px(ITEM_ICON_SIZE),
                            height: px(ITEM_ICON_SIZE),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }
                CreativeCatalogItem::Tool(tool) => {
                    slot.spawn((
                        typography::caption(tool.name.text(items.language)),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                }
            }
        });
}

fn spawn_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    selected: bool,
    hotbar: &PlayerHotbar,
    items: &mut InventoryItemView<'_>,
) {
    let (background, border) = surface::hud_control_static(selected);
    let item = hotbar.inventory_item_at(index);

    parent
        .spawn((
            Button,
            InventorySlot { index, item },
            Node {
                width: px(SLOT_SIZE),
                height: px(SLOT_SIZE),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(4)),
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

pub(super) fn spawn_inventory_item(
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
        let material = items.icon_materials.add(BlockIconMaterial::from_block(
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

    if let Some(tool) = items.tools.get(item_id) {
        slot.spawn((
            typography::caption(tool.name.text(items.language)),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ));
        return;
    }

    panic!("inventory references missing item: {item_id}");
}

fn filtered_creative_catalog<'a>(
    blocks: &'a BlockRegistry,
    tools: &'a ToolRegistry,
    categories: &InventoryCategoryRegistry,
    query: &str,
    category: Option<&str>,
    language: Language,
) -> Vec<CreativeCatalogItem<'a>> {
    let query = query.trim().to_lowercase();
    let mut catalog = blocks
        .iter()
        .map(CreativeCatalogItem::Block)
        .chain(tools.iter().map(CreativeCatalogItem::Tool))
        .filter(|item| category.map_or(true, |category| item.category() == category))
        .filter(|item| query.is_empty() || item.name(language).to_lowercase().contains(&query))
        .collect::<Vec<_>>();

    catalog.sort_by(|left, right| {
        category_order(categories, left.category())
            .cmp(&category_order(categories, right.category()))
            .then_with(|| {
                left.name(language)
                    .to_lowercase()
                    .cmp(&right.name(language).to_lowercase())
            })
            .then_with(|| left.id().cmp(right.id()))
    });
    catalog
}

fn category_order(categories: &InventoryCategoryRegistry, category: &str) -> u16 {
    categories
        .get(category)
        .map_or(u16::MAX, |definition| definition.order)
}

fn creative_grid_width() -> f32 {
    CREATIVE_COLUMNS as f32 * SLOT_SIZE + (CREATIVE_COLUMNS - 1) as f32 * SLOT_GAP
}

fn creative_content_width() -> f32 {
    CATEGORY_WIDTH
        + SCROLLBAR_TOTAL_WIDTH
        + CATEGORY_GAP
        + creative_grid_width()
        + SCROLLBAR_TOTAL_WIDTH
}
