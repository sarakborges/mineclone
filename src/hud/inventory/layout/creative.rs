use bevy::{
    prelude::*,
    text::{EditableText, FontWeight},
};

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry},
        block_id::intern_block_id,
        inventory_category::{InventoryCategoryDefinition, InventoryCategoryRegistry},
        tool::{ToolDefinition, ToolRegistry},
        tool_id::intern_tool_id,
    },
    localization::{Language, UiLocalization},
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    ui::{scrollbar, selectable, surface, text_input, typography},
};

use crate::hud::{block_icon::BlockIconMaterial, tool_icon::spawn_tool_icon};

use super::{
    InventoryItemView, InventoryLayoutState,
    super::state::{
        CATEGORY_GAP, CATEGORY_ICON_SIZE, CATEGORY_ROW_HEIGHT, CATEGORY_WIDTH, CREATIVE_COLUMNS,
        CREATIVE_GRID_HEIGHT, CreativeCatalogScrollArea, CreativeCatalogScrollbar,
        CreativeCategoryButton, CreativeCategoryScrollArea, CreativeCategoryScrollbar,
        CreativeInventorySlot, CreativeInventoryView, CreativeSearchBar, CreativeSearchText,
        ITEM_ICON_SIZE, PANEL_PADDING, SCROLLBAR_TOTAL_WIDTH, SEARCH_GAP, SEARCH_HEIGHT, SLOT_GAP,
        SLOT_SIZE,
    },
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

pub(super) fn spawn_creative_panel(
    root: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
    root.spawn(surface::hud_container(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(SEARCH_GAP),
        padding: UiRect::all(px(PANEL_PADDING)),
        border: UiRect::all(px(2)),
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
                    width: px(creative_content_width()),
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
    let placeholder_visible =
        creative_view.search_query().is_empty() && !creative_view.search_focused();
    let placeholder = localization.text(language, "inventory.searchPlaceholder");

    parent
        .spawn((
            Button,
            CreativeSearchBar,
            EditableText {
                max_characters: Some(128),
                ..EditableText::new(creative_view.search_query())
            },
            text_input::editor_style(17.0, FontWeight::NORMAL),
            Node {
                width: px(creative_content_width()),
                height: px(SEARCH_HEIGHT),
                padding: UiRect::horizontal(px(12)),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                ..default()
            },
            text_input::frame_surface(creative_view.search_focused()),
        ))
        .with_children(|search| {
            search.spawn((
                CreativeSearchText,
                typography::hud(placeholder),
                if placeholder_visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
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
    let (background, border) = selectable::static_colors(selected);

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

fn spawn_creative_slot(
    parent: &mut ChildSpawnerCommands,
    item: Option<CreativeCatalogItem<'_>>,
    selected_item: Option<&'static str>,
    items: &mut InventoryItemView<'_>,
) {
    let item_id = item.map(CreativeCatalogItem::interned_id);
    let selected = item_id.is_some() && item_id == selected_item;
    let (background, border) = selectable::static_colors(selected);

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
                    spawn_tool_icon(
                        slot,
                        tool,
                        items.asset_server,
                        items.brush_mode,
                        items.dyes,
                        items.language,
                        ITEM_ICON_SIZE,
                    );
                }
            }
        });
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
        .filter(|item| category.is_none_or(|category| item.category() == category))
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
