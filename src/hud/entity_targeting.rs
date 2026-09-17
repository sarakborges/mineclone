mod portrait;

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
};

use crate::{
    app::game_state::GameState,
    ui::surface,
};

use super::{
    HudSettings, TargetBlockPosition,
    entity_card::{EntityCard, EntityCardSource, spawn_entity_card, sync_entity_cards},
};
use portrait::PortraitCamera;

const TARGET_CROSSHAIR_OFFSET: f32 = 62.0;
const TARGET_CORNER_MARGIN: f32 = 18.0;
const PORTRAIT_RENDER_LAYER: usize = 2;

#[derive(Component)]
struct EntityHudRoot;

#[derive(Component)]
struct EntityHudRow;

pub(super) struct EntityHudPlugin;

impl Plugin for EntityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_target_entity_hud)
            .add_systems(
                Update,
                (
                    sync_target_entity_layout,
                    portrait::sync_portrait,
                    portrait::prepare_portrait,
                )
                    .chain()
                    .after(sync_entity_cards)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn entity_hud_node(position: TargetBlockPosition) -> Node {
    match position {
        TargetBlockPosition::Center | TargetBlockPosition::Hidden => Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        TargetBlockPosition::TopRight => Node {
            position_type: PositionType::Absolute,
            right: px(TARGET_CORNER_MARGIN),
            top: px(TARGET_CORNER_MARGIN),
            ..default()
        },
    }
}

fn entity_row_node(position: TargetBlockPosition) -> Node {
    Node {
        position_type: PositionType::Relative,
        bottom: if position == TargetBlockPosition::Center {
            px(TARGET_CROSSHAIR_OFFSET)
        } else {
            Val::Auto
        },
        ..default()
    }
}

fn spawn_target_entity_hud(
    mut commands: Commands,
    settings: Res<HudSettings>,
    mut images: ResMut<Assets<Image>>,
) {
    let image = images.add(Image::new_target_texture(
        128,
        128,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    commands.spawn((
        PortraitCamera,
        Camera3d::default(),
        Camera {
            order: -1,
            is_active: false,
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        },
        RenderTarget::Image(image.clone().into()),
        Transform::from_xyz(1.5, 0.8, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(PORTRAIT_RENDER_LAYER),
        DespawnOnExit(GameState::Gameplay),
    ));
    commands.spawn((
        PointLight {
            intensity: 1_500.0,
            range: 8.0,
            ..default()
        },
        Transform::from_xyz(1.6, 2.4, 2.5),
        RenderLayers::layer(PORTRAIT_RENDER_LAYER),
        DespawnOnExit(GameState::Gameplay),
    ));

    let position = settings.target_block_position();
    commands
        .spawn((
            EntityHudRoot,
            entity_hud_node(position),
            Visibility::Hidden,
            GlobalZIndex(10),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((EntityHudRow, entity_row_node(position), Pickable::IGNORE))
                .with_children(|row| {
                    spawn_entity_card(row, EntityCardSource::Target, Some(image));
                });
        });
}

fn sync_target_entity_layout(
    settings: Res<HudSettings>,
    mut root: Single<
        (&mut Node, &mut Visibility),
        (With<EntityHudRoot>, Without<EntityHudRow>),
    >,
    mut row: Single<&mut Node, (With<EntityHudRow>, Without<EntityHudRoot>)>,
    cards: Query<&EntityCard>,
) {
    let position = settings.target_block_position();
    if settings.is_changed() {
        *root.0 = entity_hud_node(position);
        **row = entity_row_node(position);
    }
    let target_present = cards
        .iter()
        .any(|card| card.source == EntityCardSource::Target && card.entity.is_some());
    let desired = if position != TargetBlockPosition::Hidden && target_present {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *root.1 != desired {
        *root.1 = desired;
    }
}
