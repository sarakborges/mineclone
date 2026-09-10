use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    player::hotbar::{HOTBAR_SLOT_COUNT, PlayerHotbar},
    ui::{theme, typography},
};

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 32.0;

#[derive(Component)]
struct HotbarSlot {
    index: usize,
}

#[derive(Component)]
struct HotbarItemName;

pub struct HotbarHudPlugin;

impl Plugin for HotbarHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_hotbar)
            .add_systems(Update, update_hotbar.run_if(in_state(GameState::Gameplay)));
    }
}

fn spawn_hotbar(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    blocks: Res<BlockRegistry>,
    hotbar: Res<PlayerHotbar>,
) {
    let selected_name = hotbar
        .item_at(hotbar.selected_slot())
        .and_then(|block_id| blocks.get(block_id))
        .map_or("", |block| block.name.as_str());

    commands
        .spawn((
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
                TextShadow {
                    offset: Vec2::new(2.0, 2.0),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.95),
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
                        let Some(block_id) = hotbar.item_at(index) else {
                            return;
                        };
                        let block = blocks.get(block_id).unwrap_or_else(|| {
                            panic!("hotbar references missing block: {block_id}")
                        });

                        if block.textures.top.is_empty() {
                            slot.spawn((
                                Node {
                                    width: px(ITEM_ICON_SIZE),
                                    height: px(ITEM_ICON_SIZE),
                                    ..default()
                                },
                                BackgroundColor(Color::WHITE),
                                Pickable::IGNORE,
                            ));
                        } else {
                            slot.spawn((
                                ImageNode::new(asset_server.load(block.textures.top.clone())),
                                Node {
                                    width: px(ITEM_ICON_SIZE),
                                    height: px(ITEM_ICON_SIZE),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                        }
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
        item_name.0 = selected_name.to_owned();
    }
}
