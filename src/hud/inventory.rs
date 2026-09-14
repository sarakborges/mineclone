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
        game_mode::GameMode,
        hotbar::{HOTBAR_INVENTORY_OFFSET, HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::{CreativeInventoryView, InventoryCursor, InventoryState},
    },
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    ui::{scrollbar, surface, theme, typography},
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
const CATEGORY_ROW_HEIGHT: f32 = SLOT_SIZE;
const CATEGORY_ICON_SIZE: f32 = 28.0;
const CATEGORY_GAP: f32 = 12.0;
const CREATIVE_VISIBLE_ROWS: usize = 5;
const CREATIVE_COLUMNS: usize = 9;
const SCROLLBAR_TOTAL_WIDTH: f32 = 14.0;
const TRASH_GAP: f32 = 10.0;
const CREATIVE_GRID_HEIGHT: f32 =
    CREATIVE_VISIBLE_ROWS as f32 * SLOT_SIZE + (CREATIVE_VISIBLE_ROWS - 1) as f32 * SLOT_GAP;

#[derive(Component)]
struct InventoryHudRoot;

#[derive(Component)]
struct InventorySlot {
    index: usize,
}

#[derive(Component)]
struct InventoryTrashButton;

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
struct CreativeCategoryScrollArea;

#[derive(Component)]
struct CreativeCatalogScrollArea;

#[derive(Component)]
struct CreativeCategoryScrollbar;

#[derive(Component)]
struct CreativeCatalogScrollbar;

#[derive(Component)]
struct InventoryCursorIcon;

#[derive(Resource, Default)]
struct CreativeScrollState {
    category_y: f32,
    catalog_y: f32,
}

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
        app.init_resource::<CreativeScrollState>()
            .add_systems(
                OnEnter(InventoryState::Open),
                spawn_inventory.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(OnExit(InventoryState::Open), reset_creative_scroll_state)
            .add_systems(
                Update,
                (
                    remember_creative_scroll_positions,
                    handle_search_focus,
                    handle_search_input,
                    handle_category_clicks,
                    handle_creative_scroll,
                    handle_creative_slot_clicks,
                    handle_slot_clicks,
                    handle_inventory_trash_clicks,
                    handle_empty_inventory_click,
                    rebuild_inventory_when_changed,
                    style_category_buttons,
                    style_creative_slots,
                    style_inventory_slots,
                    style_inventory_trash_button,
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
    scroll_state: Res<CreativeScrollState>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    player: Single<(&Transform, &GameMode), With<GameplayCamera>>,
    window: Single<&Window>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let (player_transform, game_mode) = *player;
    spawn_inventory_root(
        &mut commands,
        &asset_server,
        &blocks,
        &tools,
        &categories,
        &biomes,
        &biome_field,
        Vec2::new(player_transform.translation.x, player_transform.translation.z),
        *game_mode,
        &hotbar,
        &cursor,
        &creative_view,
        &scroll_state,
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
    mut scroll_state: ResMut<CreativeScrollState>,
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
            scroll_state.catalog_y = 0.0;
            continue;
        }

        if let Some(text) = &event.text {
            creative_view.push_search_text(text);
            scroll_state.catalog_y = 0.0;
        }
    }
}

fn handle_category_clicks(
    mut creative_view: ResMut<CreativeInventoryView>,
    mut scroll_state: ResMut<CreativeScrollState>,
    categories: Query<(&Interaction, &CreativeCategoryButton), Changed<Interaction>>,
) {
    for (interaction, category) in &categories {
        if *interaction != Interaction::Pressed {
            continue;
        }

        creative_view.blur_search();
        creative_view.select_category(category.id.as_deref());
        scroll_state.catalog_y = 0.0;
        break;
    }
}

fn handle_creative_scroll(
    mut wheel: MessageReader<MouseWheel>,
    category_buttons: Query<&Interaction, With<CreativeCategoryButton>>,
    category_scrollbars: Query<&Interaction, With<CreativeCategoryScrollbar>>,
    mut category_scroll: Query<
        &mut ScrollPosition,
        (With<CreativeCategoryScrollArea>, Without<CreativeCatalogScrollArea>),
    >,
    mut catalog_scroll: Query<
        &mut ScrollPosition,
        (With<CreativeCatalogScrollArea>, Without<CreativeCategoryScrollArea>),
    >,
) {
    let mut delta = 0.0;
    for event in wheel.read() {
        delta += match event.unit {
            MouseScrollUnit::Line => -event.y * (SLOT_SIZE + SLOT_GAP),
            MouseScrollUnit::Pixel => -event.y,
        };
    }

    if delta == 0.0 {
        return;
    }

    let pointer_is_over_categories = category_buttons
        .iter()
        .chain(category_scrollbars.iter())
        .any(|interaction| *interaction != Interaction::None);

    if pointer_is_over_categories {
        for mut position in &mut category_scroll {
            position.0.y = (position.0.y + delta).max(0.0);
        }
    } else {
        for mut position in &mut catalog_scroll {
            position.0.y = (position.0.y + delta).max(0.0);
        }
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

fn handle_inventory_trash_clicks(
    mut cursor: ResMut<InventoryCursor>,
    buttons: Query<&Interaction, (With<InventoryTrashButton>, Changed<Interaction>)>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            cursor.discard();
            break;
        }
    }
}

fn handle_empty_inventory_click(
    mouse: Res<ButtonInput<MouseButton>>,
    categories: Query<&Interaction, With<CreativeCategoryButton>>,
    creative_slots: Query<&Interaction, With<CreativeInventorySlot>>,
    inventory_slots: Query<&Interaction, With<InventorySlot>>,
    search_bars: Query<&Interaction, With<CreativeSearchBar>>,
    trash_buttons: Query<&Interaction, With<InventoryTrashButton>>,
    category_scrollbars: Query<&Interaction, With<CreativeCategoryScrollbar>>,
    catalog_scrollbars: Query<&Interaction, With<CreativeCatalogScrollbar>>,
    mut cursor: ResMut<InventoryCursor>,
) {
    if !mouse.just_pressed(MouseButton::Left) || cursor.item().is_none() {
        return;
    }

    let pointer_is_over_control = categories
        .iter()
        .chain(creative_slots.iter())
        .chain(inventory_slots.iter())
        .chain(search_bars.iter())
        .chain(trash_buttons.iter())
        .chain(category_scrollbars.iter())
        .chain(catalog_scrollbars.iter())
        .any(|interaction| *interaction != Interaction::None);

    if !pointer_is_over_control {
        cursor.discard();
    }
}

fn remember_creative_scroll_positions(
    category_scroll: Query<&ScrollPosition, With<CreativeCategoryScrollArea>>,
    catalog_scroll: Query<&ScrollPosition, With<CreativeCatalogScrollArea>>,
    mut state: ResMut<CreativeScrollState>,
) {
    if let Some(position) = category_scroll.iter().next() {
        state.category_y = position.0.y;
    }
    if let Some(position) = catalog_scroll.iter().next() {
        state.catalog_y = position.0.y;
    }
}

fn reset_creative_scroll_state(mut state: ResMut<CreativeScrollState>) {
    *state = default();
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
    scroll_state: Res<CreativeScrollState>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    player: Single<(&Transform, &GameMode), With<GameplayCamera>>,
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

    let (player_transform, game_mode) = *player;
    spawn_inventory_root(
        &mut commands,
        &asset_server,
        &blocks,
        &tools,
        &categories,
        &biomes,
        &biome_field,
        Vec2::new(player_transform.translation.x, player_transform.translation.z),
        *game_mode,
        &hotbar,
        &cursor,
        &creative_view,
        &scroll_state,
        &localization,
        active_language.get(),
        None,
        &mut icon_materials,
    );
}

fn style_category_buttons(
    creative_view: Res<CreativeInventoryView>,
    mut buttons: Query<
        (
            &Interaction,
            &CreativeCategoryButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    for (interaction, category, mut background, mut border) in &mut buttons {
        let selected = creative_view.selected_category() == category.id.as_deref();
        apply_button_visual(*interaction, selected, &mut background, &mut border);
    }
}

fn style_creative_slots(
    cursor: Res<InventoryCursor>,
    mut slots: Query<
        (
            &Interaction,
            &CreativeInventorySlot,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    for (interaction, slot, mut background, mut border) in &mut slots {
        let selected = slot.item.is_some() && slot.item == cursor.item();
        apply_button_visual(*interaction, selected, &mut background, &mut border);
    }
}

fn style_inventory_slots(
    hotbar: Res<PlayerHotbar>,
    mut slots: Query<
        (
            &Interaction,
            &InventorySlot,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    let selected_index = HOTBAR_INVENTORY_OFFSET + hotbar.selected_slot();
    for (interaction, slot, mut background, mut border) in &mut slots {
        apply_button_visual(
            *interaction,
            slot.index == selected_index,
            &mut background,
            &mut border,
        );
    }
}

fn style_inventory_trash_button(
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (With<Button>, With<InventoryTrashButton>),
    >,
) {
    for (interaction, mut background, mut border) in &mut buttons {
        let (background_color, border_color) = surface::hud_danger_control_colors(*interaction);
        background.0 = background_color;
        *border = BorderColor::all(border_color);
    }
}

fn apply_button_visual(
    interaction: Interaction,
    selected: bool,
    background: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    let (background_color, border_color) = surface::hud_control_colors(interaction, selected);
    background.0 = background_color;
    *border = BorderColor::all(border_color);
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
    game_mode: GameMode,
    hotbar: &PlayerHotbar,
    cursor: &InventoryCursor,
    creative_view: &CreativeInventoryView,
    scroll_state: &CreativeScrollState,
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
            if game_mode.has_creative_inventory() {
                spawn_creative_panel(
                    root,
                    asset_server,
                    blocks,
                    tools,
                    categories,
                    biomes,
                    biome_field,
                    player_position,
                    cursor,
                    creative_view,
                    scroll_state,
                    localization,
                    language,
                    icon_materials,
                );
            }
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
    cursor: &InventoryCursor,
    creative_view: &CreativeInventoryView,
    scroll_state: &CreativeScrollState,
    localization: &UiLocalization,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let catalog = filtered_creative_catalog(
        blocks,
        tools,
        categories,
        creative_view.search_query(),
        creative_view.selected_category(),
        language,
    );

    root.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(SEARCH_GAP),
            padding: UiRect::all(px(PANEL_PADDING)),
            ..default()
        },
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
                    scroll_state.category_y,
                    localization,
                    language,
                    icon_materials,
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
                                ScrollPosition(Vec2::new(0.0, scroll_state.catalog_y)),
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
                                spawn_creative_grid(
                                    scroll,
                                    &catalog,
                                    cursor.item(),
                                    asset_server,
                                    biomes,
                                    biome_field,
                                    player_position,
                                    language,
                                    icon_materials,
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
            search.spawn((typography::hud(search_text), Pickable::IGNORE));
        });
}

fn spawn_category_list(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    categories: &InventoryCategoryRegistry,
    creative_view: &CreativeInventoryView,
    initial_scroll_y: f32,
    localization: &UiLocalization,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
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
                        asset_server,
                        blocks,
                        localization,
                        language,
                        icon_materials,
                    );

                    for category in ordered {
                        let selected =
                            creative_view.selected_category() == Some(category.id.as_str());
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
                let block = blocks.get(&block_icon.block).unwrap_or_else(|| {
                    panic!(
                        "inventory category {} references missing block icon {}",
                        category.id, block_icon.block
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

            button.spawn((typography::inventory_category(label), Pickable::IGNORE));
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_creative_grid(
    parent: &mut ChildSpawnerCommands,
    catalog: &[CreativeCatalogItem<'_>],
    selected_item: Option<&'static str>,
    asset_server: &AssetServer,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
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
                    spawn_creative_slot(
                        row_node,
                        item,
                        selected_item,
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
            ..default()
        },
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
                    position_type: PositionType::Relative,
                    width: px(creative_grid_width()),
                    height: px(SLOT_SIZE),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|hotbar_anchor| {
                hotbar_anchor
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

                spawn_inventory_trash_button(hotbar_anchor);
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
                position_type: PositionType::Absolute,
                left: px(creative_grid_width() + TRASH_GAP),
                top: px(0),
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

#[allow(clippy::too_many_arguments)]
fn spawn_creative_slot(
    parent: &mut ChildSpawnerCommands,
    item: Option<CreativeCatalogItem<'_>>,
    selected_item: Option<&'static str>,
    asset_server: &AssetServer,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    language: Language,
    icon_materials: &mut Assets<BlockIconMaterial>,
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
    let (background, border) = surface::hud_control_static(selected);

    parent
        .spawn((
            Button,
            InventorySlot { index },
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
