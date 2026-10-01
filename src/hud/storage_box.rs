use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        builtin_ids::{BUCKET_FLUID_METADATA_KEY, WATER_FLUID_ID},
        item::ItemRegistry,
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
    ui::{selectable, surface, typography},
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
}

#[derive(SystemParam)]
struct StorageSpawnContext<'w, 's> {
    storage: Res<'w, StorageBoxStorage>,
    hotbar: Res<'w, PlayerHotbar>,
    cursor: Res<'w, InventoryCursor>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    window: Single<'w, 's, &'static Window>,
    content: StorageItemContent<'w>,
    icon_materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

pub(super) struct StorageBoxHudPlugin;

impl Plugin for StorageBoxHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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
                handle_storage_slot_clicks,
                handle_player_slot_clicks,
                rebuild_storage_box_when_changed,
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
                panel.spawn((typography::hud_heading("Storage Box"), Pickable::IGNORE));
                spawn_storage_grid(panel, &context.storage, &mut items);
                panel.spawn((typography::hud_heading("Inventory"), Pickable::IGNORE));
                spawn_player_inventory(panel, &context.hotbar, &mut items);
            });

            if let Some(stack) = context.cursor.stack() {
                spawn_cursor(root, stack, cursor_position, &mut items);
            }
        });
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

fn handle_storage_slot_clicks(
    interactions: Query<(&Interaction, &StorageBoxSlot), Changed<Interaction>>,
    mut storage: ResMut<StorageBoxStorage>,
    mut cursor: ResMut<InventoryCursor>,
) {
    for (interaction, slot) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
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
) {
    for (interaction, slot) in &interactions {
        if *interaction == Interaction::Pressed {
            cursor.click_slot(&mut hotbar, slot.0);
            return;
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
) {
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
