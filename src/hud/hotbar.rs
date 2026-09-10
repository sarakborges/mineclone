use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry},
    hud::block_icon::BlockIconMaterial,
    player::{
        camera::GameplayCamera,
        hotbar::{HOTBAR_SLOT_COUNT, PlayerHotbar},
    },
    rendering::{
        block_model::BlockModelInstance,
        block_tint::block_tint_at,
    },
    ui::{theme, typography},
    world::biome_field::BiomeField,
};

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 34.0;

#[derive(Component)]
struct HotbarHudRoot;

#[derive(Component)]
struct HotbarSlot {
    index: usize,
}

#[derive(Component)]
struct HotbarItemName;

#[derive(SystemParam)]
struct HotbarHudContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    hotbar: Res<'w, PlayerHotbar>,
}

pub struct HotbarHudPlugin;

impl Plugin for HotbarHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_hotbar)
            .add_systems(
                Update,
                (update_hotbar, update_hotbar_item_tints).run_if(in_state(GameState::Gameplay)),
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

    let selected_name = content
        .hotbar
        .item_at(content.hotbar.selected_slot())
        .and_then(|block_id| content.blocks.get(block_id))
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
                HotbarItemName,
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
                    let selected = index == content.hotbar.selected_slot();
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
                        HotbarSlot { index },
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
                        let Some(block_id) = content.hotbar.item_at(index) else {
                            return;
                        };
                        let block = content.blocks.get(block_id).unwrap_or_else(|| {
                            panic!("hotbar references missing block: {block_id}")
                        });
                        let material = icon_materials.add(BlockIconMaterial::from_block(
                            block,
                            &content.asset_server,
                            Color::WHITE,
                        ));

                        slot.spawn((
                            BlockModelInstance::new(block_id),
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

fn update_hotbar(
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    mut slots: Query<(&HotbarSlot, &mut BackgroundColor, &mut BorderColor)>,
    mut item_name: Single<&mut Text, With<HotbarItemName>>,
) {
    if !hotbar.is_changed() {
        return;
    }

    for (slot, mut background, mut border) in &mut slots {
        let selected = slot.index == hotbar.selected_slot();

        background.0 = if selected {
            Color::srgba(0.08, 0.07, 0.16, 0.94)
        } else {
            theme::HUD_SURFACE
        };
        *border = BorderColor::all(if selected {
            theme::TEXT_PRIMARY
        } else {
            Color::srgba(0.70, 0.72, 0.82, 0.28)
        });
    }

    let selected_name = hotbar
        .item_at(hotbar.selected_slot())
        .and_then(|block_id| blocks.get(block_id))
        .map_or("", |block| block.name.as_str());

    if item_name.0 != selected_name {
        item_name.0.clear();
        item_name.0.push_str(selected_name);
    }
}

fn update_hotbar_item_tints(
    player: Single<&Transform, With<GameplayCamera>>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    icons: Query<(&BlockModelInstance, &MaterialNode<BlockIconMaterial>)>,
    mut materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let position = Vec2::new(player.translation.x, player.translation.z);

    for (model, material_handle) in &icons {
        let Some(block_id) = model.block_id() else {
            continue;
        };
        let tint = block_tint_at(block_id, position, &biome_field, &biomes);
        let Some(mut material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        material.set_tint(tint);
    }
}
