use bevy::{
    ecs::system::SystemParam,
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::{EditableText, FontWeight},
};

use crate::{
    app::game_state::GameState,
    content::{
        builtin_ids::{BUCKET_FLUID_METADATA_KEY, WATER_FLUID_ID},
        item::{ItemRegistry, display_name},
        layer::LayerRegistry,
        object::ObjectRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
    },
    gameplay::{
        modal::GameplayModalState,
        storage_box::{STORAGE_BOX_SLOT_COUNT, StorageBoxStorage},
    },
    localization::ActiveLanguage,
    player::{
        camera::GameplayCamera,
        hotbar::{BACKPACK_SLOT_COUNT, HOTBAR_INVENTORY_OFFSET, HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::InventoryCursor,
        item_stack::ItemStack,
    },
    rendering::block_visual_content::BlockVisualContent,
    tools::BrushMode,
    ui::{
        button::{self, ButtonVariant},
        selectable, surface, text_input, theme, typography,
    },
};

use super::{
    block_icon::BlockIconMaterial,
    inventory::{INVENTORY_PANEL_BORDER_WIDTH, INVENTORY_SLOT_GAP, INVENTORY_SLOT_SIZE},
    item_icon::{HudItemIconView, spawn_hud_item_icon},
    item_stack_count::spawn_item_stack_count,
};

const ITEM_ICON_SIZE: f32 = 30.0;
const PANEL_PADDING: f32 = 18.0;
const SECTION_GAP: f32 = 18.0;
const SEARCH_HEIGHT: f32 = 40.0;
const SEARCH_WIDTH: f32 = 210.0;
const HEADER_GAP: f32 = 8.0;
const BUCKET_TOOL_ID: &str = "asteria:bucket";
const LAVA_FLUID_ID: &str = "asteria:lava";
const BUCKET_EMPTY_ICON: &str = "textures/tools/iron_bucket_empty.png";
const BUCKET_WATER_ICON: &str = "textures/tools/iron_bucket_water.png";
const BUCKET_LAVA_ICON: &str = "textures/tools/iron_bucket_lava.png";

#[derive(Component)]
struct StorageBoxHudRoot;

#[derive(Component)]
struct StorageBoxSlot(usize);

#[derive(Component)]
struct StoragePlayerSlot(usize);

#[derive(Component)]
struct StorageCursorIcon;

#[derive(Component)]
struct StorageSearchFrame;

#[derive(Component)]
struct StorageSearchBar;

#[derive(Component)]
struct StorageSearchText;

#[derive(Component)]
struct StorageSortButton;

#[derive(Component)]
struct StorageSortTooltip;

#[derive(Resource, Default)]
struct StorageBoxView {
    search: String,
    search_focused: bool,
}

impl StorageBoxView {
    fn search_query(&self) -> &str {
        &self.search
    }

    fn search_focused(&self) -> bool {
        self.search_focused
    }

    fn focus_search(&mut self) {
        self.search_focused = true;
    }

    fn blur_search(&mut self) {
        self.search_focused = false;
    }

    fn set_search_query(&mut self, query: String) {
        if self.search != query {
            self.search = query;
        }
    }
}

#[derive(SystemParam)]
struct StorageItemContent<'w> {
    visual: BlockVisualContent<'w>,
    items: Res<'w, ItemRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
    dyes: Res<'w, SecondaryPropertyRegistry>,
    brush_mode: Res<'w, BrushMode>,
    language: Res<'w, ActiveLanguage>,
}

impl StorageItemContent<'_> {
    fn view<'a>(
        &'a self,
        player_position: Vec2,
        icon_materials: &'a mut Assets<BlockIconMaterial>,
    ) -> HudItemIconView<'a> {
        HudItemIconView {
            asset_server: &self.visual.asset_server,
            items: &self.items,
            blocks: &self.visual.blocks,
            layers: &self.layers,
            objects: &self.objects,
            tools: &self.tools,
            dyes: &self.dyes,
            brush_mode: &self.brush_mode,
            biomes: &self.visual.biomes,
            biome_field: &self.visual.biome_field,
            player_position,
            language: self.language.get(),
            icon_materials,
        }
    }

    fn item_name<'a>(&'a self, item_id: &'a str) -> &'a str {
        display_name(
            item_id,
            &self.items,
            &self.visual.blocks,
            &self.layers,
            &self.objects,
            &self.tools,
            self.language.get(),
        )
    }
}

#[derive(SystemParam)]
struct StorageSpawnContext<'w, 's> {
    storage: Res<'w, StorageBoxStorage>,
    hotbar: Res<'w, PlayerHotbar>,
    cursor: Res<'w, InventoryCursor>,
    view: Res<'w, StorageBoxView>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    window: Single<'w, 's, &'static Window>,
    content: StorageItemContent<'w>,
    icon_materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

pub(super) struct StorageBoxHudPlugin;

impl Plugin for StorageBoxHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StorageBoxView>()
            .add_systems(
                OnEnter(GameplayModalState::StorageBox),
                spawn_storage_box_hud.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameplayModalState::StorageBox),
                close_storage_box,
            )
            .add_systems(
                Update,
                (
                    handle_storage_search_focus,
                    handle_storage_search_input,
                    handle_storage_sort_clicks,
                    handle_storage_slot_clicks,
                    handle_player_slot_clicks,
                    sync_storage_search_focus,
                    rebuild_storage_box_when_changed,
                    style_storage_search_field,
                    style_storage_slots,
                    sync_storage_sort_tooltip,
                    update_storage_cursor_position,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayModalState::StorageBox)),
            );
    }
}

fn spawn_storage_box_hud(mut commands: Commands, mut context: StorageSpawnContext) {
    spawn_storage_box_root(&mut commands, &mut context);
}

fn rebuild_storage_box_when_changed(
    mut commands: Commands,
    roots: Query<Entity, With<StorageBoxHudRoot>>,
    mut context: StorageSpawnContext,
) {
    if !context.storage.is_changed() && !context.hotbar.is_changed() && !context.cursor.is_changed() {
        return;
    }

    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_storage_box_root(&mut commands, &mut context);
}

fn spawn_storage_box_root(commands: &mut Commands, context: &mut StorageSpawnContext<'_, '_>) {
    if context.storage.active_position().is_none() {
        return;
    }

    let player_position = Vec2::new(context.player.translation.x, context.player.translation.z);
    let mut items = context
        .content
        .view(player_position, &mut context.icon_materials);
    let cursor_position = context.window.cursor_position().unwrap_or(Vec2::ZERO);

    commands
        .spawn((
            StorageBoxHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
            DespawnOnExit(GameplayModalState::StorageBox),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn(surface::hud_container(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: px(SECTION_GAP),
                padding: UiRect::all(px(PANEL_PADDING)),
                border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
                ..default()
            }))
            .insert(Pickable::IGNORE)
            .with_children(|panel| {
                spawn_storage_header(panel, &context.view);
                spawn_storage_grid(panel, &context.storage, &mut items);
                panel.spawn((typography::hud_heading("Inventory"), Pickable::IGNORE));
                spawn_player_inventory(panel, &context.hotbar, &mut items);
            });

            if let Some(stack) = context.cursor.stack() {
                spawn_cursor(root, stack, cursor_position, &mut items);
            }
        });
}

fn spawn_storage_header(parent: &mut ChildSpawnerCommands, view: &StorageBoxView) {
    parent
        .spawn((
            Node {
                width: px(storage_grid_width()),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(HEADER_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|header| {
            header.spawn((typography::hud_heading("Storage Box"), Pickable::IGNORE));
            header
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(HEADER_GAP),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|controls| {
                    spawn_storage_search_field(controls, view);
                    spawn_storage_sort_button(controls);
                });
        });
}

fn spawn_storage_search_field(parent: &mut ChildSpawnerCommands, view: &StorageBoxView) {
    let placeholder_visible = view.search_query().is_empty() && !view.search_focused();
    parent
        .spawn((
            Button,
            StorageSearchFrame,
            Node {
                position_type: PositionType::Relative,
                width: px(SEARCH_WIDTH),
                height: px(SEARCH_HEIGHT),
                padding: UiRect::horizontal(px(text_input::INPUT_PADDING_X)),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            text_input::frame_surface(view.search_focused()),
        ))
        .with_children(|frame| {
            frame.spawn((
                StorageSearchBar,
                EditableText {
                    max_characters: Some(128),
                    ..EditableText::new(view.search_query())
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
                StorageSearchText,
                typography::hud("Search..."),
                TextColor(theme::TEXT_MUTED),
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

fn spawn_storage_sort_button(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(button::icon_button(
            StorageSortButton,
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
                    StorageSortTooltip,
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
                        typography::caption("Organize storage"),
                        TextLayout::no_wrap(),
                        Pickable::IGNORE,
                    ));
                });
        });
}

fn storage_grid_width() -> f32 {
    HOTBAR_SLOT_COUNT as f32 * INVENTORY_SLOT_SIZE
        + (HOTBAR_SLOT_COUNT - 1) as f32 * INVENTORY_SLOT_GAP
}

fn spawn_storage_grid(
    parent: &mut ChildSpawnerCommands,
    storage: &StorageBoxStorage,
    items: &mut HudItemIconView<'_>,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(INVENTORY_SLOT_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|grid| {
            for row in 0..3 {
                grid.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(INVENTORY_SLOT_GAP),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row_node| {
                    for column in 0..HOTBAR_SLOT_COUNT {
                        let index = row * HOTBAR_SLOT_COUNT + column;
                        debug_assert!(index < STORAGE_BOX_SLOT_COUNT);
                        spawn_storage_slot(row_node, index, storage.active_stack_at(index), items);
                    }
                });
            }
        });
}

fn spawn_player_inventory(
    parent: &mut ChildSpawnerCommands,
    hotbar: &PlayerHotbar,
    items: &mut HudItemIconView<'_>,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(INVENTORY_SLOT_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|inventory| {
            for row in 0..3 {
                inventory
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(INVENTORY_SLOT_GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|row_node| {
                        for column in 0..HOTBAR_SLOT_COUNT {
                            let index = row * HOTBAR_SLOT_COUNT + column;
                            debug_assert!(index < BACKPACK_SLOT_COUNT);
                            spawn_player_slot(row_node, index, hotbar.inventory_stack_at(index), items);
                        }
                    });
            }

            inventory
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(INVENTORY_SLOT_GAP),
                        margin: UiRect::top(px(INVENTORY_SLOT_GAP * 2.0)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|hotbar_row| {
                    for column in 0..HOTBAR_SLOT_COUNT {
                        let index = HOTBAR_INVENTORY_OFFSET + column;
                        spawn_player_slot(hotbar_row, index, hotbar.inventory_stack_at(index), items);
                    }
                });
        });
}

fn spawn_storage_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    stack: Option<&ItemStack>,
    items: &mut HudItemIconView<'_>,
) {
    let (background, border) = selectable::static_colors(false);
    parent
        .spawn((
            Button,
            StorageBoxSlot(index),
            slot_node(),
            BackgroundColor(background),
            BorderColor::all(border),
        ))
        .with_children(|slot| spawn_stack(slot, stack, items));
}

fn spawn_player_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    stack: Option<&ItemStack>,
    items: &mut HudItemIconView<'_>,
) {
    let (background, border) = selectable::static_colors(false);
    parent
        .spawn((
            Button,
            StoragePlayerSlot(index),
            slot_node(),
            BackgroundColor(background),
            BorderColor::all(border),
        ))
        .with_children(|slot| spawn_stack(slot, stack, items));
}

fn slot_node() -> Node {
    Node {
        width: px(INVENTORY_SLOT_SIZE),
        height: px(INVENTORY_SLOT_SIZE),
        border: UiRect::all(px(2)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn spawn_stack(
    parent: &mut ChildSpawnerCommands,
    stack: Option<&ItemStack>,
    items: &mut HudItemIconView<'_>,
) {
    let Some(stack) = stack else {
        return;
    };
    spawn_hud_item_icon(
        parent,
        stack.id(),
        items,
        ITEM_ICON_SIZE,
        bucket_icon_for_stack(stack),
        None,
    );
    spawn_item_stack_count(parent, stack.quantity());
}

fn spawn_cursor(
    parent: &mut ChildSpawnerCommands,
    stack: &ItemStack,
    position: Vec2,
    items: &mut HudItemIconView<'_>,
) {
    parent
        .spawn((
            StorageCursorIcon,
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
            GlobalZIndex(200),
            Pickable::IGNORE,
        ))
        .with_children(|cursor| {
            spawn_hud_item_icon(
                cursor,
                stack.id(),
                items,
                ITEM_ICON_SIZE,
                bucket_icon_for_stack(stack),
                None,
            );
            spawn_item_stack_count(cursor, stack.quantity());
        });
}

fn handle_storage_search_focus(
    frames: Query<&Interaction, (With<StorageSearchFrame>, Changed<Interaction>)>,
    editor: Query<Entity, With<StorageSearchBar>>,
    mut focus: ResMut<InputFocus>,
    mut view: ResMut<StorageBoxView>,
) {
    if frames
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
        && let Ok(entity) = editor.single()
    {
        view.focus_search();
        focus.set(entity, FocusCause::Pressed);
    }
}

fn handle_storage_search_input(
    editor: Query<(Entity, &EditableText), With<StorageSearchBar>>,
    focus: Res<InputFocus>,
    mut view: ResMut<StorageBoxView>,
) {
    let Ok((entity, editable)) = editor.single() else {
        return;
    };
    let focused = focus.get() == Some(entity);
    if focused && !view.search_focused() {
        view.focus_search();
    } else if !focused && view.search_focused() {
        view.blur_search();
    }

    let next = text_input::editable_value(editable);
    if next != view.search_query() {
        view.set_search_query(next);
    }
}

fn sync_storage_search_focus(
    view: Res<StorageBoxView>,
    mut focus: ResMut<InputFocus>,
    editor: Query<Entity, With<StorageSearchBar>>,
) {
    if !view.search_focused()
        && let Ok(entity) = editor.single()
        && focus.get() == Some(entity)
    {
        focus.clear();
    }
}

fn handle_storage_sort_clicks(
    buttons: Query<&Interaction, (With<StorageSortButton>, Changed<Interaction>)>,
    mut storage: ResMut<StorageBoxStorage>,
    mut view: ResMut<StorageBoxView>,
) {
    if !buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    let mut stacks = Vec::new();
    for index in 0..STORAGE_BOX_SLOT_COUNT {
        if let Some(slot) = storage.active_slot_mut(index)
            && let Some(stack) = slot.take()
        {
            stacks.push(stack);
        }
    }
    stacks.sort_by(ItemStack::stacking_cmp);

    let mut compacted: Vec<ItemStack> = Vec::with_capacity(stacks.len());
    for stack in stacks {
        let remainder = if let Some(last) = compacted.last_mut()
            && last.can_stack_with(&stack)
        {
            last.merge_from(stack)
        } else {
            Some(stack)
        };
        if let Some(remainder) = remainder {
            compacted.push(remainder);
        }
    }

    let mut compacted = compacted.into_iter();
    for index in 0..STORAGE_BOX_SLOT_COUNT {
        if let Some(slot) = storage.active_slot_mut(index) {
            *slot = compacted.next();
        }
    }
    view.blur_search();
}

fn handle_storage_slot_clicks(
    interactions: Query<(&Interaction, &StorageBoxSlot), Changed<Interaction>>,
    mut storage: ResMut<StorageBoxStorage>,
    mut cursor: ResMut<InventoryCursor>,
    mut view: ResMut<StorageBoxView>,
) {
    for (interaction, slot) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        view.blur_search();
        if let Some(item_slot) = storage.active_slot_mut(slot.0) {
            cursor.click_item_slot(item_slot);
        }
        return;
    }
}

fn handle_player_slot_clicks(
    interactions: Query<(&Interaction, &StoragePlayerSlot), Changed<Interaction>>,
    mut hotbar: ResMut<PlayerHotbar>,
    mut cursor: ResMut<InventoryCursor>,
    mut view: ResMut<StorageBoxView>,
) {
    for (interaction, slot) in &interactions {
        if *interaction == Interaction::Pressed {
            view.blur_search();
            cursor.click_slot(&mut hotbar, slot.0);
            return;
        }
    }
}

fn style_storage_search_field(
    view: Res<StorageBoxView>,
    mut frames: Query<(&mut BackgroundColor, &mut BorderColor), With<StorageSearchFrame>>,
    mut placeholders: Query<(&mut Visibility, &mut TextColor), With<StorageSearchText>>,
) {
    let fill = BackgroundColor(text_input::INPUT_FILL);
    let border = BorderColor::all(text_input::input_border(view.search_focused()));
    for (mut background, mut current_border) in &mut frames {
        if *background != fill {
            *background = fill;
        }
        if *current_border != border {
            *current_border = border;
        }
    }

    let next_visibility = if view.search_query().is_empty() && !view.search_focused() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut visibility, mut color) in &mut placeholders {
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
        if color.0 != theme::TEXT_MUTED {
            color.0 = theme::TEXT_MUTED;
        }
    }
}

fn style_storage_slots(
    content: StorageItemContent,
    storage: Res<StorageBoxStorage>,
    view: Res<StorageBoxView>,
    mut slots: Query<
        (
            Ref<Interaction>,
            &StorageBoxSlot,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    let selection_changed = storage.is_changed() || view.is_changed();
    let query = view.search_query().trim().to_lowercase();

    for (interaction, slot, background, border) in &mut slots {
        if !selection_changed && !interaction.is_changed() {
            continue;
        }

        let search_match = !query.is_empty()
            && storage.active_stack_at(slot.0).is_some_and(|stack| {
                let item_id = stack.id();
                item_id.to_lowercase().contains(&query)
                    || content.item_name(item_id).to_lowercase().contains(&query)
            });
        selectable::apply_colors(
            selectable::colors(*interaction, search_match),
            background,
            border,
        );
    }
}

fn sync_storage_sort_tooltip(
    buttons: Query<&Interaction, (With<StorageSortButton>, Changed<Interaction>)>,
    mut tooltip: Single<
        &mut Visibility,
        (With<StorageSortTooltip>, Without<StorageSortButton>),
    >,
) {
    for interaction in &buttons {
        let next = if *interaction == Interaction::None {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if **tooltip != next {
            **tooltip = next;
        }
    }
}

fn update_storage_cursor_position(
    window: Single<&Window>,
    mut cursors: Query<&mut Node, With<StorageCursorIcon>>,
) {
    let Some(position) = window.cursor_position() else {
        return;
    };
    for mut node in &mut cursors {
        node.left = px(position.x - ITEM_ICON_SIZE * 0.5);
        node.top = px(position.y - ITEM_ICON_SIZE * 0.5);
    }
}

fn close_storage_box(
    mut storage: ResMut<StorageBoxStorage>,
    mut hotbar: ResMut<PlayerHotbar>,
    mut cursor: ResMut<InventoryCursor>,
    mut view: ResMut<StorageBoxView>,
) {
    view.blur_search();
    if let Some(stack) = cursor.take_stack() {
        let remainder = storage.try_insert_active(stack).err();
        let remainder = remainder.and_then(|stack| hotbar.try_insert_stack(stack).err());
        if let Some(stack) = remainder {
            cursor.set_stack(Some(stack));
        }
    }
    storage.close();
}

fn bucket_icon_for_stack(stack: &ItemStack) -> Option<&'static str> {
    if stack.id() != BUCKET_TOOL_ID {
        return None;
    }
    Some(match stack.metadata().get(BUCKET_FLUID_METADATA_KEY) {
        Some(WATER_FLUID_ID) => BUCKET_WATER_ICON,
        Some(LAVA_FLUID_ID) => BUCKET_LAVA_ICON,
        _ => BUCKET_EMPTY_ICON,
    })
}
