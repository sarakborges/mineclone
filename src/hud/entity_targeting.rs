mod portrait;

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::system::SystemParam,
    prelude::*,
    render::render_resource::TextureFormat,
};

use crate::{
    app::game_state::GameState,
    content::creature::CreatureRegistry,
    creatures::CreatureInstance,
    localization::ActiveLanguage,
    targeting::block::{BlockTargetingSet, TargetedCreature},
    ui::{surface, typography},
};

use super::{HudSettings, TargetBlockPosition};
use portrait::PortraitCamera;

const TARGET_SLOT_SIZE: f32 = 44.0;
const TARGET_ICON_SIZE: f32 = 34.0;
const TARGET_CROSSHAIR_OFFSET: f32 = 62.0;
const TARGET_CORNER_MARGIN: f32 = 18.0;
const PORTRAIT_RENDER_LAYER: usize = 2;

#[derive(Component)]
struct EntityHudRoot;

#[derive(Component)]
struct EntityHudRow;

#[derive(Component)]
struct EntityHudText;

type EntityHudRootView<'w, 's> = Single<
    'w,
    's,
    (&'static mut Node, &'static mut Visibility),
    (With<EntityHudRoot>, Without<EntityHudRow>),
>;

type EntityHudRowView<'w, 's> = Single<
    'w,
    's,
    &'static mut Node,
    (With<EntityHudRow>, Without<EntityHudRoot>),
>;

pub(super) struct EntityHudPlugin;

impl Plugin for EntityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_entity_hud)
            .add_systems(
                Update,
                (
                    update_entity_hud,
                    portrait::sync_portrait,
                    portrait::prepare_portrait,
                )
                    .chain()
                    .after(BlockTargetingSet::Raycast)
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
        align_items: AlignItems::Center,
        column_gap: px(10),
        ..default()
    }
}

fn spawn_entity_hud(
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
    let (background, border) = surface::hud_control_static(false);
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
                    row.spawn((
                        Node {
                            width: px(TARGET_SLOT_SIZE),
                            height: px(TARGET_SLOT_SIZE),
                            min_width: px(TARGET_SLOT_SIZE),
                            min_height: px(TARGET_SLOT_SIZE),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(4)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(background),
                        BorderColor::all(border),
                        Pickable::IGNORE,
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            ImageNode::new(image),
                            Node {
                                width: px(TARGET_ICON_SIZE),
                                height: px(TARGET_ICON_SIZE),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    });
                    row.spawn((
                        EntityHudText,
                        typography::hud(""),
                        typography::tooltip_shadow(),
                        Pickable::IGNORE,
                    ));
                });
        });
}

#[derive(SystemParam)]
struct EntityHudContext<'w, 's> {
    targeted: Res<'w, TargetedCreature>,
    settings: Res<'w, HudSettings>,
    creatures: Query<'w, 's, &'static CreatureInstance>,
    definitions: Res<'w, CreatureRegistry>,
    language: Res<'w, ActiveLanguage>,
}

fn update_entity_hud(
    context: EntityHudContext,
    mut root: EntityHudRootView,
    mut row: EntityHudRowView,
    mut text: Single<&mut Text, With<EntityHudText>>,
) {
    let position = context.settings.target_block_position();
    let visible_target = if position != TargetBlockPosition::Hidden {
        context.targeted.0.and_then(|entity| context.creatures.get(entity).ok())
    } else {
        None
    };
    let (root_node, visibility) = &mut *root;
    let desired = if visible_target.is_some() { Visibility::Visible } else { Visibility::Hidden };
    if **visibility != desired {
        **visibility = desired;
    }
    if context.settings.is_changed() {
        **root_node = entity_hud_node(position);
        **row = entity_row_node(position);
    }
    let Some(creature) = visible_target else {
        return;
    };
    let name = context.definitions.get(&creature.definition_id)
        .map_or(creature.definition_id.as_str(), |definition| definition.name.text(context.language.get()));
    if text.0 != name {
        text.0 = name.to_owned();
    }
}
