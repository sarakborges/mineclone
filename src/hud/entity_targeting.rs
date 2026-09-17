use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::creature::CreatureRegistry,
    creatures::CreatureInstance,
    localization::{ActiveLanguage, UiLocalization},
    targeting::block::{BlockTargetingSet, TargetedCreature},
    ui::typography,
};

use super::{HudSettings, TargetBlockPosition};

const TARGET_CROSSHAIR_OFFSET: f32 = 62.0;
const TARGET_CORNER_MARGIN: f32 = 18.0;

#[derive(Component)]
struct EntityHudRoot;

#[derive(Component)]
struct EntityHudText;

pub(super) struct EntityHudPlugin;

impl Plugin for EntityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_entity_hud)
            .add_systems(
                Update,
                update_entity_hud
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

fn spawn_entity_hud(mut commands: Commands, settings: Res<HudSettings>) {
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
            root.spawn((
                Node {
                    position_type: PositionType::Relative,
                    bottom: if position == TargetBlockPosition::Center {
                        px(TARGET_CROSSHAIR_OFFSET)
                    } else {
                        Val::Auto
                    },
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    EntityHudText,
                    typography::hud(""),
                    typography::tooltip_shadow(),
                    Pickable::IGNORE,
                ));
            });
        });
}

fn update_entity_hud(
    targeted: Res<TargetedCreature>,
    settings: Res<HudSettings>,
    creatures: Query<(&CreatureInstance, &Transform)>,
    definitions: Res<CreatureRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut root: Single<(&mut Node, &mut Visibility, &Children), With<EntityHudRoot>>,
    mut nodes: Query<&mut Node>,
    mut text: Single<&mut Text, With<EntityHudText>>,
) {
    let (root_node, visibility, children) = &mut *root;
    let position = settings.target_block_position();
    let visible_target = if position != TargetBlockPosition::Hidden {
        targeted.0.and_then(|entity| creatures.get(entity).ok())
    } else {
        None
    };
    let desired = if visible_target.is_some() { Visibility::Visible } else { Visibility::Hidden };
    if **visibility != desired {
        **visibility = desired;
    }
    if settings.is_changed() {
        **root_node = entity_hud_node(position);
        if let Some(child) = children.first()
            && let Ok(mut node) = nodes.get_mut(*child)
        {
            node.bottom = if position == TargetBlockPosition::Center {
                px(TARGET_CROSSHAIR_OFFSET)
            } else {
                Val::Auto
            };
        }
    }
    let Some((creature, transform)) = visible_target else {
        return;
    };
    let language = language.get();
    let name = definitions.get(&creature.definition_id)
        .map_or(creature.definition_id.as_str(), |definition| definition.name.text(language));
    let voxel = transform.translation.floor().as_ivec3();
    let coordinates = localization.text(language, "hud.coordinates")
        .replace("{x}", &voxel.x.to_string())
        .replace("{y}", &voxel.y.to_string())
        .replace("{z}", &voxel.z.to_string());
    let next = format!("{name}\n{}\n{coordinates}", creature.definition_id);
    if text.0 != next {
        text.0 = next;
    }
}
