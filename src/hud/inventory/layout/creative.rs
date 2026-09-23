use bevy::{
    prelude::*,
    text::{EditableText, FontWeight},
};

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry},
        block_id::intern_block_id,
        inventory_category::{InventoryCategoryDefinition, InventoryCategoryRegistry},
        item::{ItemDefinition, ItemRegistry},
        item_id::intern_item_id,
        layer::{LayerDefinition, LayerRegistry},
        layer_id::intern_layer_id,
        tool::{ToolDefinition, ToolRegistry},
        tool_id::intern_tool_id,
    },
    localization::{Language, UiLocalization},
    ui::{scrollbar, selectable, surface, text_input, typography},
};

use super::{
    InventoryItemView, InventoryLayoutState,
    item::spawn_inventory_item,
    super::state::{
        CATEGORY_GAP, CATEGORY_ICON_SIZE, CATEGORY_ROW_HEIGHT, CATEGORY_WIDTH,
        CREATIVE_CATEGORY_HEIGHT, CREATIVE_COLUMNS, CREATIVE_GRID_HEIGHT,
        CreativeCatalogScrollArea, CreativeCatalogScrollbar, CreativeCategoryButton,
        CreativeCategoryScrollArea, CreativeCategoryScrollbar, CreativeInventorySlot,
        CreativeInventoryView, CreativeSearchBar, CreativeSearchText, PANEL_BORDER_WIDTH,
        PANEL_PADDING, PLAYER_HEADER_GAP, PLAYER_SEARCH_WIDTH, SCROLLBAR_TOTAL_WIDTH,
        SEARCH_HEIGHT, SECTION_GAP, SLOT_GAP, SLOT_SIZE,
    },
};

#[derive(Clone, Copy)]
struct CreativeCatalogSources<'a> {
    items: &'a ItemRegistry,
    blocks: &'a BlockRegistry,
    layers: &'a LayerRegistry,
    tools: &'a ToolRegistry,
    categories: &'a InventoryCategoryRegistry,
    language: Language,
}

#[derive(Clone, Copy)]
enum CreativeCatalogItem<'a> {
    Item(&'a ItemDefinition),
    Block(&'a BlockDefinition),
    Layer(&'a LayerDefinition),
    Tool(&'a ToolDefinition),
}

impl<'a> CreativeCatalogItem<'a> {
    fn id(self) -> &'a str {
        match self {
            Self::Item(item) => &item.id,
            Self::Block(block) => &block.id,
            Self::Layer(layer) => &layer.id,
            Self::Tool(tool) => &tool.id,
        }
    }

    fn category(self) -> &'a str {
        match self {
            Self::Item(item) => &item.category,
            Self::Block(block) => &block.category,
            Self::Layer(layer) => &layer.category,
            Self::Tool(tool) => &tool.category,
        }
    }

    fn name(self, language: Language) -> &'a str {
        match self {
            Self::Item(item) => item.name.text(language),
            Self::Block(block) => block.name.text(language),
            Self::Layer(layer) => layer.name.text(language),
            Self::Tool(tool) => tool.name.text(language),
        }
    }

    fn interned_id(self) -> &'static str {
        match self {
            Self::Item(item) => intern_item_id(&item.id),
            Self::Block(block) => intern_block_id(&block.id),
            Self::Layer(layer) => intern_layer_id(&layer.id),
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
        align_items: AlignItems::FlexStart,
        row_gap: px(SECTION_GAP),
        padding: UiRect::all(px(PANEL_PADDING)),
        border: UiRect::all(px(PANEL_BORDER_WIDTH)),
        ..default()
    }))
    .insert(Pickable::IGNORE)
    .with_children(|panel| {
        spawn_creative_header(panel, state, items);

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
                            width: px(super::player::player_panel_content_width()),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            row_gap: px(SECTION_GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|right_column| {
                        spawn_creative_catalog(right_column, state, items);
                        super::player::spawn_player_hotbar_footer(
                            right_column,
                            state.hotbar,
                            items,
                        );
                    });
            });
    });
}

fn spawn_creative_header(
    parent: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &InventoryItemView<'_>,
) {
    parent
        .spawn((
            Node {
                width: px(creative_content_width()),
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
            spawn_search_bar(
                header,
                state.creative_view,
                state.localization,
                items.language,
            );
        });
}

fn spawn_creative_catalog(
    parent: &mut ChildSpawnerCommands,
    state: &InventoryLayoutState<'_>,
    items: &mut InventoryItemView<'_>,
) {
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
                .spawn(scrollbar::vertical_scrollbar_always(scroll_area))
                .insert(CreativeCatalogScrollbar);
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
                width: px(PLAYER_SEARCH_WIDTH),
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
                height: px(CREATIVE_CATEGORY_HEIGHT),
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
                        height: px(CREATIVE_CATEGORY_HEIGHT),
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
                .spawn(scrollbar::vertical_scrollbar_always(scroll_area))
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
            let icon = category.map_or(
                "textures/inventory/categories/everything.png",
                |category| category.icon.as_str(),
            );
            button.spawn((
                ImageNode::new(items.asset_server.load(icon.to_owned())),
                Node {
                    width: px(CATEGORY_ICON_SIZE),
                    height: px(CATEGORY_ICON_SIZE),
                    ..default()
                },
                Pickable::IGNORE,
            ));

            button.spawn((typography::inventory_category(label), Pickable::IGNORE));
        });
}

pub(in crate::hud::inventory) fn spawn_creative_catalog_rows(
    parent: &mut ChildSpawnerCommands,
    categories: &InventoryCategoryRegistry,
    creative_view: &CreativeInventoryView,
    selected_item: Option<&'static str>,
    items: &mut InventoryItemView<'_>,
) {
    let catalog = filtered_creative_catalog(
        CreativeCatalogSources {
            items: items.items,
            blocks: items.blocks,
            layers: items.layers,
            tools: items.tools,
            categories,
            language: items.language,
        },
        creative_view.search_query(),
        creative_view.selected_category(),
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
            if let Some(item_id) = item_id {
                spawn_inventory_item(slot, item_id, items);
            }
        });
}

fn filtered_creative_catalog<'a>(
    sources: CreativeCatalogSources<'a>,
    query: &str,
    category: Option<&str>,
) -> Vec<CreativeCatalogItem<'a>> {
    let query = query.trim().to_lowercase();
    let mut catalog = sources
        .items
        .iter()
        .map(CreativeCatalogItem::Item)
        .chain(sources.blocks.iter().map(CreativeCatalogItem::Block))
        .chain(sources.layers.iter().map(CreativeCatalogItem::Layer))
        .chain(sources.tools.iter().map(CreativeCatalogItem::Tool))
        .filter(|item| category.is_none_or(|category| item.category() == category))
        .filter(|item| {
            query.is_empty()
                || item
                    .name(sources.language)
                    .to_lowercase()
                    .contains(&query)
        })
        .collect::<Vec<_>>();

    catalog.sort_by(|left, right| {
        category_order(sources.categories, left.category())
            .cmp(&category_order(sources.categories, right.category()))
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
        + super::player::player_panel_content_width()
}
