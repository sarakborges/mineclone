use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry},
    player::{
        camera::GameplayCamera,
        hotbar::{HOTBAR_INVENTORY_OFFSET, HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::{InventoryCursor, InventoryState},
    },
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    ui::theme,
    world::biome_field::BiomeField,
};

use super::block_icon::BlockIconMaterial;

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 34.0;
const SLOT_GAP: f32 = 4.0;
const SECTION_GAP: f32 = 18.0;
const PANEL_PADDING: f32 = 18.0;

#[derive(Component)]
struct InventoryHudRoot;

#[derive(Component)]
struct InventorySlot {
    index: usize,
}

#[derive(Component)]
struct InventoryCursorIcon;

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
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    hotbar: Res<PlayerHotbar>,
    cursor: Res<InventoryCursor>,
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
        &biomes,
        &biome_field,
        Vec2::new(player.translation.x, player.translation.z),
        &hotbar,
        &cursor,
        window.cursor_position(),
        &mut icon_materials,
    );
}

fn handle_slot_clicks(
    mut hotbar: ResMut<PlayerHotbar>,
    mut cursor: ResMut<InventoryCursor>,
    slots: Query<(&Interaction, &InventorySlot), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }

        cursor.click_slot(&mut hotbar, slot.index);
        break;
    }
}

fn rebuild_inventory_when_changed(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    blocks: Res<BlockRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    hotbar: Res<PlayerHotbar>,
    cursor: Res<InventoryCursor>,
    player: Single<&Transform, With<GameplayCamera>>,
    window: Single<&Window>,
    roots: Query<Entity, With<InventoryHudRoot>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !hotbar.is_changed() && !cursor.is_changed() {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    spawn_inventory_root(
        &mut commands,
        &asset_server,
        &blocks,
        &biomes,
        &biome_field,
        Vec2::new(player.translation.x, player.translation.z),
        &hotbar,
        &cursor,
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
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
    hotbar: &PlayerHotbar,
    cursor: &InventoryCursor,
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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
            DespawnOnExit(InventoryState::Open),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
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
                                            biomes,
                                            biome_field,
                                            player_position,
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
                                biomes,
                                biome_field,
                                player_position,
                                icon_materials,
                            );
                        }
                    });
            });

            let Some(block_id) = cursor.item() else {
                return;
            };
            let block = blocks
                .get(block_id)
                .unwrap_or_else(|| panic!("inventory cursor references missing block: {block_id}"));
            let tint = block_tint_at(block.tint, player_position, biome_field, biomes);
            let material = icon_materials.add(BlockIconMaterial::from_block(
                block,
                asset_server,
                tint,
            ));
            let position = cursor_position.unwrap_or(Vec2::ZERO);

            root.spawn((
                InventoryCursorIcon,
                BlockModel::display(block_id),
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
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    player_position: Vec2,
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
            let Some(block_id) = hotbar.inventory_item_at(index) else {
                return;
            };
            let block = blocks
                .get(block_id)
                .unwrap_or_else(|| panic!("inventory references missing block: {block_id}"));
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
        });
}
