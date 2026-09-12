use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry, block::BlockRegistry, block_orientation::BlockOrientation,
    },
    hud::block_icon::BlockIconMaterial,
    player::{
        camera::GameplayCamera,
        hotbar::{HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::InventoryState,
    },
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    targeting::{PlacementOrientation, block::BlockTargetingSet},
    ui::{theme, typography},
    world::biome_field::BiomeField,
};

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 34.0;

#[derive(Component)]
struct HotbarHudRoot;

#[derive(Component)]
struct HotbarBlockModel {
    index: usize,
    orientation: BlockOrientation,
}

#[derive(SystemParam)]
struct HotbarHudContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    hotbar: Res<'w, PlayerHotbar>,
    inventory_state: Res<'w, State<InventoryState>>,
}

pub struct HotbarHudPlugin;

impl Plugin for HotbarHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_hotbar)
            .add_systems(
                Update,
                refresh_hotbar.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                update_hotbar_item_visuals
                    .after(BlockTargetingSet::PlacementState)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(InventoryState::Open),
                hide_hotbar.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(InventoryState::Closed),
                show_hotbar.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_hotbar(
    mut commands: Commands,
    content: HotbarHudContent,
    existing: Query<(), With<HotbarHudRoot>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !existing.is_empty() {
        return;
    }

    let visibility = if *content.inventory_state.get() == InventoryState::Open {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };

    spawn_hotbar_root(
        &mut commands,
        &content.asset_server,
        &content.blocks,
        &content.hotbar,
        visibility,
        &mut icon_materials,
    );
}

fn refresh_hotbar(
    mut commands: Commands,
    content: HotbarHudContent,
    roots: Query<Entity, With<HotbarHudRoot>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !content.hotbar.is_changed() {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let visibility = if *content.inventory_state.get() == InventoryState::Open {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };

    spawn_hotbar_root(
        &mut commands,
        &content.asset_server,
        &content.blocks,
        &content.hotbar,
        visibility,
        &mut icon_materials,
    );
}

fn spawn_hotbar_root(
    commands: &mut Commands,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    hotbar: &PlayerHotbar,
    visibility: Visibility,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let selected_name = hotbar
        .item_at(hotbar.selected_slot())
        .and_then(|block_id| blocks.get(block_id))
        .map_or("", |block| block.name.as_str());

    commands
        .spawn((
            HotbarHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                bottom: px(18),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(8),
                ..default()
            },
            visibility,
            GlobalZIndex(10),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                typography::hud(selected_name),
                TextLayout::justify(Justify::Center),
                Node {
                    min_height: px(18),
                    ..default()
                },
            ));

            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                padding: UiRect::all(px(4)),
                ..default()
            })
            .with_children(|row| {
                for index in 0..HOTBAR_SLOT_COUNT {
                    let selected = index == hotbar.selected_slot();
                    let border_color = if selected {
                        theme::TEXT_PRIMARY
                    } else {
                        Color::srgba(0.70, 0.72, 0.82, 0.28)
                    };
                    let background = if selected {
                        Color::srgba(0.08, 0.07, 0.16, 0.94)
                    } else {
                        theme::HUD_SURFACE
                    };

                    row.spawn((
                        Node {
                            width: px(SLOT_SIZE),
                            height: px(SLOT_SIZE),
                            border: UiRect::all(px(2)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(background),
                        BorderColor::all(border_color),
                        Pickable::IGNORE,
                    ))
                    .with_children(|slot| {
                        let Some(block_id) = hotbar.item_at(index) else {
                            return;
                        };
                        let block = blocks.get(block_id).unwrap_or_else(|| {
                            panic!("hotbar references missing block: {block_id}")
                        });
                        let orientation = block.default_orientation();
                        let material = icon_materials.add(BlockIconMaterial::from_block(
                            block,
                            asset_server,
                            Color::WHITE,
                        ));

                        slot.spawn((
                            HotbarBlockModel { index, orientation },
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
            });
        });
}

fn hide_hotbar(mut roots: Query<&mut Visibility, With<HotbarHudRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Hidden;
    }
}

fn show_hotbar(mut roots: Query<&mut Visibility, With<HotbarHudRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Visible;
    }
}

fn update_hotbar_item_visuals(
    player: Single<&Transform, With<GameplayCamera>>,
    asset_server: Res<AssetServer>,
    blocks: Res<BlockRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    placement_orientation: Res<PlacementOrientation>,
    mut icons: Query<(
        &BlockModel,
        &mut HotbarBlockModel,
        &MaterialNode<BlockIconMaterial>,
    )>,
    mut materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let position = Vec2::new(player.translation.x, player.translation.z);

    for (model, mut icon, material_handle) in &mut icons {
        let Some(block_id) = model.block_id() else {
            continue;
        };
        let block = blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));
        let orientation = placement_orientation.for_block(icon.index, block);
        let tint = block_tint_at(block.tint, position, &biome_field, &biomes);
        let Some(mut material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        if icon.orientation != orientation {
            material.set_block_orientation(block, orientation, &asset_server);
            icon.orientation = orientation;
        }
        material.set_tint(tint);
    }
}
