use bevy::{
    input::{
        ButtonState,
        keyboard::KeyboardInput,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
};

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
    localization::{ActiveLanguage, Language, UiLocalization},
    player::{
        camera::GameplayCamera,
        hotbar::{HOTBAR_INVENTORY_OFFSET, HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::{CreativeInventoryView, InventoryCursor, InventoryState},
    },
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    ui::{theme, typography},
    world::biome_field::BiomeField,
};

use super::block_icon::BlockIconMaterial;

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 34.0;
const SLOT_GAP: f32 = 4.0;
const SECTION_GAP: f32 = 18.0;
const PANEL_GAP: f32 = 24.0;
const PANEL_PADDING: f32 = 18.0;
const SEARCH_HEIGHT: f32 = 40.0;
const SEARCH_GAP: f32 = 14.0;
const CATEGORY_WIDTH: f32 = 172.0;
const CATEGORY_ROW_HEIGHT: f32 = 38.0;
const CATEGORY_ICON_SIZE: f32 = 28.0;
const CATEGORY_GAP: f32 = 12.0;
const CREATIVE_VISIBLE_ROWS: usize = 5;
const CREATIVE_COLUMNS: usize = 9;
const SCROLLBAR_WIDTH: f32 = 8.0;
const SCROLLBAR_GAP: f32 = 8.0;
const CREATIVE_GRID_HEIGHT: f32 =
    CREATIVE_VISIBLE_ROWS as f32 * SLOT_SIZE + (CREATIVE_VISIBLE_ROWS - 1) as f32 * SLOT_GAP;

#[derive(Component)]
struct InventoryHudRoot;

#[derive(Component)]
struct InventorySlot {
    index: usize,
}

#[derive(Component)]
struct CreativeInventorySlot {
    item: Option<&'static str>,
}

#[derive(Component)]
struct CreativeSearchBar;

#[derive(Component)]
struct CreativeCategoryButton {
    id: Option<String>,
}

#[derive(Component)]
struct InventoryCursorIcon;

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

pub(super) struct InventoryHudPlugin;

impl Plugin for InventoryHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(InventoryState::Open),
            spawn_inventory.run_if(in_state(GameState::Gameplay)),
        )
        .add_systems(
            Update,
            (
                handle_search_focus,
                handle_search_input,
                handle_category_clicks,
                handle_creative_scroll,
                handle_creative_slot_clicks,
                handle_slot_clicks,
                rebuild_inventory_when_changed,
                update_cursor_icon_position,
            )
                .chain()
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(InventoryState::Open)),
        );
    }
}

fn spawn_inventory(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    blocks: Res<BlockRegistry>,
    tools: Res<ToolRegistry>,
    categories: Res<InventoryCategoryRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    hotbar: Res<PlayerHotbar>,
    cursor: Res<InventoryCursor>,
    creative_view: Res<CreativeInventoryView>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    player: Single<&Transform, With<GameplayCamera>>,
    window: Single<&Window>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
    existing: Query<(), With<InventoryHudRoot>>,
) {
    if !existing.is_empty() {
        return;
    }

    spawn_inventory_root(
        &mut commands,
        &asset_server,
        &blocks,
        &tools,
        &categories,
        &biomes,
        &biome_field,
        Vec2::new(player.translation.x, player.translation.z),
        &hotbar,
        &cursor,
        &creative_view,
        &localization,
        active_language.get(),
        window.cursor_position(),
        &mut icon_materials,
    );
}

fn handle_search_focus(
    mut creative_view: ResMut<CreativeInventoryView>,
    search_bars: Query<&Interaction, (With<CreativeSearchBar>, Changed<Interaction>)>,
) {
    for interaction in &search_bars {
        if *interaction == Interaction::Pressed {
            creative_view.focus_search();
            break;
        }
    }
}

fn handle_search_input(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut creative_view: ResMut<CreativeInventoryView>,
) {
    if !creative_view.search_focused() {
        keyboard_input.clear();
        return;
    }

    for event in keyboard_input.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        if event.key_code == KeyCode::Backspace {
            creative_view.backspace_search();
            continue;
        }

        if let Some(text) = &event.text {
            creative_view.push_search_text(text);
        }
    }
}

fn handle_category_clicks(
    mut creative_view: ResMut<CreativeInventoryView>,
    categories: Query<(&Interaction, &CreativeCategoryButton), Changed<Interaction>>,
) {
    for (interaction, category) in &categories {
        if *interaction != Interaction::Pressed {
            continue;
        }

        creative_view.blur_search();
        creative_view.select_category(category.id.as_deref());
        break;
    }
}

fn handle_creative_scroll(
    mut wheel: MessageReader<MouseWheel>,
    blocks: Res<BlockRegistry>,
    tools: Res<ToolRegistry>,
    active_language: Res<ActiveLanguage>,
    mut creative_view: ResMut<CreativeInventoryView>,
) {
    let mut delta = 0.0;
    for event in wheel.read() {
        delta += match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 40.0,
        };
    }

    if delta == 0.0 {
        return;
    }

    let total_rows = creative_total_rows(
        &blocks,
        &tools,
        creative_view.search_query(),
        creative_view.selected_category(),
        active_language.get(),
    );
    let max_scroll = total_rows.saturating_sub(CREATIVE_VISIBLE_ROWS);
    if max_scroll == 0 {
        if creative_view.scroll_row() != 0 {
            creative_view.set_scroll_row(0);
        }
        return;
    }

    let steps = delta.abs().ceil().max(1.0) as usize;
    let next = if delta < 0.0 {
        (creative_view.scroll_row() + steps).min(max_scroll)
    } else {
        creative_view.scroll_row().saturating_sub(steps)
    };

    if next != creative_view.scroll_row() {
        creative_view.set_scroll_row(next);
    }
}

fn handle_creative_slot_clicks(
    mut cursor: ResMut<InventoryCursor>,
    mut creative_view: ResMut<CreativeInventoryView>,
    slots: Query<(&Interaction, &CreativeInventorySlot), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }

        creative_view.blur_search();
        if let Some(item) = slot.item {
            cursor.pick_creative_item(item);
        }
        break;
    }
}

fn handle_slot_clicks(
    mut hotbar: ResMut<PlayerHotbar>,
    mut cursor: ResMut<InventoryCursor>,
    mut creative_view: ResMut<CreativeInventoryView>,
    slots: Query<(&Interaction, &InventorySlot), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }

        creative_view.blur_search();
        cursor.click_slot(&mut hotbar, slot.index);
        break;
    }
}

fn rebuild_inventory_when_changed(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    blocks: Res<BlockRegistry>,
    tools: Res<ToolRegistry>,
    categories: Res<InventoryCategoryRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    hotbar: Res<PlayerHotbar>,
    cursor: Res<InventoryCursor>,
    creative_view: Res<CreativeInventoryView>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    player: Single<&Transform, With<GameplayCamera>>,
    window: Single<&Window>,
    roots: Query<Entity, With<InventoryHudRoot>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !hotbar.is_changed()
        && !cursor.is_changed()
        && !creative_view.is_changed()
        && !active_language.is_changed()
    {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    spawn_inventory_root(
        &mut commands,
        &asset_server,
        &blocks,
        &tools,
        &categories,
        &biomes,
        &biome_field,
        Vec2::new(player.translation.x, player.translation.z),
        &hotbar,
        &cursor,
        &creative_view,
        &localization,
        active_language.get(),
        window.cursor_position(),
        &mut icon_materials,
    );
}

fn update_cursor_icon_position(
    window: Single<&Window>,
    mut icons: Query<&mut Node, With<InventoryCursorIcon>>,
) {
    let Some(position) = window.cursor_position() else {
        return;
    };

    for mut node in &mut icons {
        node.left = px(position.x - ITEM_ICON_SIZE * 0.5);
        node.top = px(position.y - ITEM_ICON_SIZE * 0.5);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_inventory_root(
    commands: &mut Commands,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    tools: &ToolRegistry,
    categories: &InventoryCategoryRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    hotbar: &PlayerHotbar,
    cursor: &InventoryCursor,
    creative_view: &CreativeInventoryView,
    localization: &UiLocalization,
    language: Language,
    cursor_position: Option<Vec2>,
    icon_materials: &mut Assets<BlockIconMaterial>,
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
            spawn_creative_panel(
                root,
                asset_server,
                blocks,
                tools,
                categories,
                biomes,
                biome_field,
                player_position,
                creative_view,
                localization,
                language,
                icon_materials,
            );
            spawn_player_inventory_panel(
                root,
                asset_server,
                blocks,
                tools,
                biomes,
                biome_field,
                player_position,
                hotbar,
                language,
                icon_materials,
            );

            let Some(item_id) = cursor.item() else {
                return;
            };
            let position = cursor_position.unwrap_or(Vec2::ZERO);

            if let Some(block) = blocks.get(item_id) {
                let tint = block_tint_at(block.tint, player_position, biome_field, biomes);
                let material = icon_materials.add(BlockIconMaterial::from_block(
                    block,
                    asset_server,
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

            if let Some(tool) = tools.get(item_id) {
                root.spawn((
                    InventoryCursorIcon,
                    typography::caption(tool.name.text(language)),
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
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_creative_panel(
    root: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    tools: &ToolRegistry,
    categories: &InventoryCategoryRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    creative_view: &CreativeInventoryView,
    localization: &UiLocalization,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let catalog = filtered_creative_catalog(
        blocks,
        tools,
        creative_view.search_query(),
        creative_view.selected_category(),
        language,
    );
    let total_rows = catalog.len().div_ceil(CREATIVE_COLUMNS).max(1);
    let max_scroll = total_rows.saturating_sub(CREATIVE_VISIBLE_ROWS);
    let scroll_row = creative_view.scroll_row().min(max_scroll);
    let first_item = scroll_row * CREATIVE_COLUMNS;

    root.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(SEARCH_GAP),
            padding: UiRect::all(px(PANEL_PADDING)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        },
        BackgroundColor(theme::FROSTED_SURFACE),
        theme::frosted_surface_gradient(),
        BorderColor::all(Color::srgba(0.70, 0.72, 0.92, 0.20)),
        Pickable::IGNORE,
    ))
    .with_children(|panel| {
        spawn_search_bar(panel, creative_view, localization, language);

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
                    asset_server,
                    blocks,
                    categories,
                    creative_view,
                    localization,
                    language,
                    icon_materials,
                );

                content
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Stretch,
                            column_gap: px(SCROLLBAR_GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|catalog_content| {
                        spawn_creative_grid(
                            catalog_content,
                            &catalog,
                            first_item,
                            asset_server,
                            biomes,
                            biome_field,
                            player_position,
                            language,
                            icon_materials,
                        );
                        spawn_creative_scrollbar(catalog_content, total_rows, scroll_row);
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
        theme::TEXT_PRIMARY
    } else {
        Color::srgba(0.70, 0.72, 0.82, 0.28)
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
                border_radius: BorderRadius::all(px(7)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.018, 0.05, 0.88)),
            BorderColor::all(search_border),
        ))
        .with_children(|search| {
            search.spawn((typography::hud(search_text), Pickable::IGNORE));
        });
}

fn spawn_category_list(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    categories: &InventoryCategoryRegistry,
    creative_view: &CreativeInventoryView,
    localization: &UiLocalization,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let mut ordered = categories.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.display_name
            .text(language)
            .to_lowercase()
            .cmp(&right.display_name.text(language).to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });

    parent
        .spawn((
            Node {
                width: px(CATEGORY_WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: px(SLOT_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|list| {
            spawn_category_button(
                list,
                None,
                creative_view.selected_category().is_none(),
                asset_server,
                blocks,
                localization,
                language,
                icon_materials,
            );

            for category in ordered {
                let selected = creative_view.selected_category() == Some(category.id.as_str());
                spawn_category_button(
                    list,
                    Some(category),
                    selected,
                    asset_server,
                    blocks,
                    localization,
                    language,
                    icon_materials,
                );
            }
        });
}

fn spawn_category_button(
    parent: &mut ChildSpawnerCommands,
    category: Option<&InventoryCategoryDefinition>,
    selected: bool,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    localization: &UiLocalization,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let id = category.map(|category| category.id.clone());
    let label = match category {
        Some(category) => category.display_name.text(language),
        None => localization.text(language, "inventory.everything"),
    };
    let border = if selected {
        theme::TEXT_PRIMARY
    } else {
        Color::srgba(0.70, 0.72, 0.82, 0.28)
    };
    let background = if selected {
        Color::srgba(0.08, 0.07, 0.16, 0.94)
    } else {
        theme::HUD_SURFACE
    };

    parent
        .spawn((
            Button,
            CreativeCategoryButton { id },
            Node {
                width: percent(100),
                height: px(CATEGORY_ROW_HEIGHT),
                padding: UiRect::horizontal(px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(border),
        ))
        .with_children(|button| {
            if let Some(category) = category {
                let block = blocks.get(&category.block_icon.block).unwrap_or_else(|| {
                    panic!(
                        "inventory category {} references missing block icon {}",
                        category.id, category.block_icon.block
                    )
                });
                let block_id = intern_block_id(&block.id);
                let material = icon_materials.add(BlockIconMaterial::from_block(
                    block,
                    asset_server,
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

            button.spawn((typography::hud(label), Pickable::IGNORE));
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_creative_grid(
    parent: &mut ChildSpawnerCommands,
    catalog: &[CreativeCatalogItem<'_>],
    first_item: usize,
    asset_server: &AssetServer,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(SLOT_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|grid| {
            for row in 0..CREATIVE_VISIBLE_ROWS {
                grid.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(SLOT_GAP),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row_node| {
                    for column in 0..CREATIVE_COLUMNS {
                        let visible_index = row * CREATIVE_COLUMNS + column;
                        let item = catalog.get(first_item + visible_index).copied();
                        spawn_creative_slot(
                            row_node,
                            item,
                            asset_server,
                            biomes,
                            biome_field,
                            player_position,
                            language,
                            icon_materials,
                        );
                    }
                });
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_player_inventory_panel(
    root: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    tools: &ToolRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    hotbar: &PlayerHotbar,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    root.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(SECTION_GAP),
            padding: UiRect::all(px(PANEL_PADDING)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        },
        BackgroundColor(theme::FROSTED_SURFACE),
        theme::frosted_surface_gradient(),
        BorderColor::all(Color::srgba(0.70, 0.72, 0.92, 0.20)),
        Pickable::IGNORE,
    ))
    .with_children(|panel| {
        panel
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
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
                                spawn_slot(
                                    row_node,
                                    index,
                                    false,
                                    hotbar,
                                    asset_server,
                                    blocks,
                                    tools,
                                    biomes,
                                    biome_field,
                                    player_position,
                                    language,
                                    icon_materials,
                                );
                            }
                        });
                }
            });

        panel
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
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
                        asset_server,
                        blocks,
                        tools,
                        biomes,
                        biome_field,
                        player_position,
                        language,
                        icon_materials,
                    );
                }
            });
    });
}

fn spawn_creative_scrollbar(
    parent: &mut ChildSpawnerCommands,
    total_rows: usize,
    scroll_row: usize,
) {
    let visible_ratio = (CREATIVE_VISIBLE_ROWS as f32 / total_rows as f32).min(1.0);
    let thumb_height = CREATIVE_GRID_HEIGHT * visible_ratio;
    let max_scroll = total_rows.saturating_sub(CREATIVE_VISIBLE_ROWS);
    let travel = CREATIVE_GRID_HEIGHT - thumb_height;
    let thumb_top = if max_scroll == 0 {
        0.0
    } else {
        travel * scroll_row as f32 / max_scroll as f32
    };

    parent
        .spawn((
            Node {
                width: px(SCROLLBAR_WIDTH),
                height: px(CREATIVE_GRID_HEIGHT),
                position_type: PositionType::Relative,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(Color::srgba(0.65, 0.68, 0.82, 0.10)),
            Pickable::IGNORE,
        ))
        .with_children(|track| {
            track.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(thumb_top),
                    width: px(SCROLLBAR_WIDTH),
                    height: px(thumb_height),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.78, 0.80, 0.94, 0.55)),
                Pickable::IGNORE,
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_creative_slot(
    parent: &mut ChildSpawnerCommands,
    item: Option<CreativeCatalogItem<'_>>,
    asset_server: &AssetServer,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let item_id = item.map(CreativeCatalogItem::interned_id);

    parent
        .spawn((
            Button,
            CreativeInventorySlot { item: item_id },
            Node {
                width: px(SLOT_SIZE),
                height: px(SLOT_SIZE),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme::HUD_SURFACE),
            BorderColor::all(Color::srgba(0.70, 0.72, 0.82, 0.28)),
        ))
        .with_children(|slot| {
            let Some(item) = item else {
                return;
            };

            match item {
                CreativeCatalogItem::Block(block) => {
                    let block_id = intern_block_id(&block.id);
                    let tint = block_tint_at(block.tint, player_position, biome_field, biomes);
                    let material = icon_materials.add(BlockIconMaterial::from_block(
                        block,
                        asset_server,
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
                        typography::caption(tool.name.text(language)),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                }
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    selected: bool,
    hotbar: &PlayerHotbar,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    tools: &ToolRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let border = if selected {
        theme::TEXT_PRIMARY
    } else {
        Color::srgba(0.70, 0.72, 0.82, 0.28)
    };
    let background = if selected {
        Color::srgba(0.08, 0.07, 0.16, 0.94)
    } else {
        theme::HUD_SURFACE
    };

    parent
        .spawn((
            Button,
            InventorySlot { index },
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
            let Some(item_id) = hotbar.inventory_item_at(index) else {
                return;
            };

            if let Some(block) = blocks.get(item_id) {
                let tint = block_tint_at(block.tint, player_position, biome_field, biomes);
                let material = icon_materials.add(BlockIconMaterial::from_block(
                    block,
                    asset_server,
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

            if let Some(tool) = tools.get(item_id) {
                slot.spawn((
                    typography::caption(tool.name.text(language)),
                    TextLayout::justify(Justify::Center),
                    Pickable::IGNORE,
                ));
                return;
            }

            panic!("inventory references missing item: {item_id}");
        });
}

fn filtered_creative_catalog<'a>(
    blocks: &'a BlockRegistry,
    tools: &'a ToolRegistry,
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
        left.name(language)
            .to_lowercase()
            .cmp(&right.name(language).to_lowercase())
            .then_with(|| left.id().cmp(right.id()))
    });
    catalog
}

fn creative_total_rows(
    blocks: &BlockRegistry,
    tools: &ToolRegistry,
    query: &str,
    category: Option<&str>,
    language: Language,
) -> usize {
    let count = filtered_creative_catalog(blocks, tools, query, category, language).len();
    count.div_ceil(CREATIVE_COLUMNS).max(1)
}

fn creative_grid_width() -> f32 {
    CREATIVE_COLUMNS as f32 * SLOT_SIZE + (CREATIVE_COLUMNS - 1) as f32 * SLOT_GAP
}

fn creative_content_width() -> f32 {
    CATEGORY_WIDTH + CATEGORY_GAP + creative_grid_width() + SCROLLBAR_GAP + SCROLLBAR_WIDTH
}
