use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::creature::CreatureRegistry,
    entity::EntityHealth,
    creatures::CreatureInstance,
    localization::ActiveLanguage,
    player::camera::GameplayCamera,
    targeting::block::TargetedCreature,
    ui::{surface, theme, typography},
};

use super::{HudSettings, TargetBlockPosition};

const AVATAR_SIZE: f32 = 64.0;
const AVATAR_IMAGE_SIZE: f32 = 54.0;
const INFO_WIDTH: f32 = 180.0;
const HEALTH_BAR_HEIGHT: f32 = 22.0;
const HEALTH_FILL_COLOR: Color = Color::srgba(0.78, 0.16, 0.25, 0.94);

/// The card itself receives an ECS Entity, not a copy of player or creature UI.
/// Source only selects which game entity is bound to each existing HUD slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntityCardSource {
    LocalPlayer,
    Target,
}

#[derive(Component)]
pub(super) struct EntityCard {
    pub(super) source: EntityCardSource,
    pub(super) entity: Option<Entity>,
}

#[derive(Component)]
pub(super) struct EntityCardName(EntityCardSource);

#[derive(Component)]
pub(super) struct EntityCardHealthFill(EntityCardSource);

#[derive(Component)]
pub(super) struct EntityCardHealthLabel(EntityCardSource);

/// Shared avatar, name and optional health layout used by both HUD placements.
/// The local player has no world avatar asset yet, so its existing '?' remains.
pub(super) fn spawn_entity_card(
    parent: &mut ChildSpawnerCommands,
    source: EntityCardSource,
    portrait: Option<Handle<Image>>,
) {
    let (avatar_background, avatar_border) = surface::hud_control_static(false);
    parent
        .spawn((
            EntityCard {
                source,
                entity: None,
            },
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(AVATAR_SIZE),
                    height: px(AVATAR_SIZE),
                    min_width: px(AVATAR_SIZE),
                    min_height: px(AVATAR_SIZE),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(4)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(avatar_background),
                BorderColor::all(avatar_border),
                Pickable::IGNORE,
            ))
            .with_children(|avatar| {
                if let Some(image) = portrait {
                    avatar.spawn((
                        ImageNode::new(image),
                        Node {
                            width: px(AVATAR_IMAGE_SIZE),
                            height: px(AVATAR_IMAGE_SIZE),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                } else {
                    avatar.spawn((
                        typography::hud_subheading("?"),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                }
            });

            row.spawn((
                Node {
                    width: px(INFO_WIDTH),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: px(8),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|info| {
                info.spawn((
                    EntityCardName(source),
                    typography::hud(""),
                    typography::tooltip_shadow(),
                    Pickable::IGNORE,
                ));

                spawn_entity_health_bar(info, source);
            });
        });
}

fn spawn_entity_health_bar(info: &mut ChildSpawnerCommands, source: EntityCardSource) {
    info.spawn((
        EntityCardHealthFill(source),
        Node {
            position_type: PositionType::Relative,
            width: percent(100),
            height: px(HEALTH_BAR_HEIGHT),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(theme::SLIDER_TRACK),
        BorderColor::all(surface::HUD_BORDER_COLOR),
        Pickable::IGNORE,
    ))
    .with_children(|health| {
        health.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(HEALTH_FILL_COLOR),
            Pickable::IGNORE,
        ));
        health.spawn((
            EntityCardHealthLabel(source),
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
            Pickable::IGNORE,
        ))
        .with_children(|label| {
            label.spawn((
                typography::inventory_category(""),
                typography::tooltip_shadow(),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ));
        });
    });
}

#[derive(SystemParam)]
pub(super) struct EntityCardSubjects<'w, 's> {
    player: Query<'w, 's, Entity, With<GameplayCamera>>,
    target: Res<'w, TargetedCreature>,
    creatures: Query<'w, 's, &'static CreatureInstance>,
    definitions: Res<'w, CreatureRegistry>,
    language: Res<'w, ActiveLanguage>,
    settings: Res<'w, HudSettings>,
}

/// Resolve both HUD slots to real entities, then update their common card view.
/// A missing/despawned target clears its name and hides its card immediately.
#[allow(clippy::type_complexity)]
pub(super) fn sync_entity_cards(
    subjects: EntityCardSubjects,
    mut queries: ParamSet<(
        Query<(&mut EntityCard, &mut Visibility)>,
        Query<(&EntityCardName, &mut Text), Without<EntityCardHealthLabel>>,
        Query<&EntityHealth>,
        Query<(&EntityCardHealthFill, &Children)>,
        Query<&mut Node, Without<EntityCardHealthFill>>,
        Query<(&EntityCardHealthLabel, &Children), Without<EntityCardName>>,
    )>,
) {
    let player_entity = subjects.player.iter().next();
    let target_entity = if subjects.settings.target_block_position() == TargetBlockPosition::Hidden {
        None
    } else {
        subjects
            .target
            .0
            .filter(|entity| subjects.creatures.get(*entity).is_ok())
    };
    let target_name = target_entity
        .and_then(|entity| subjects.creatures.get(entity).ok())
        .and_then(|creature| subjects.definitions.get(&creature.definition_id))
        .map(|definition| definition.name.text(subjects.language.get()))
        .unwrap_or("");

    for (mut card, mut visibility) in &mut queries.p0() {
        let selected_entity = match card.source {
            EntityCardSource::LocalPlayer => player_entity,
            EntityCardSource::Target => target_entity,
        };
        if card.entity != selected_entity {
            card.entity = selected_entity;
        }
        let desired = if selected_entity.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != desired {
            *visibility = desired;
        }
    }

    let player_health = player_entity.and_then(|entity| {
        queries
            .p2()
            .get(entity)
            .map(|value| (value.current(), value.max()))
            .ok()
    });
    let target_health = target_entity.and_then(|entity| {
        queries
            .p2()
            .get(entity)
            .map(|value| (value.current(), value.max()))
            .ok()
    });

    let health_fill_updates: Vec<(Entity, Val)> = {
        let mut updates = Vec::new();
        for (marker, children) in queries.p3().iter() {
            let health = match marker.0 {
                EntityCardSource::LocalPlayer => player_health,
                EntityCardSource::Target => target_health,
            };
            let fraction = health
                .map(|(current, max)| (current / max).clamp(0.0, 1.0))
                .unwrap_or(0.0);
            if let Some(&fill_entity) = children.first() {
                updates.push((fill_entity, percent(fraction * 100.0)));
            }
        }
        updates
    };

    for (fill_entity, width) in health_fill_updates {
        if let Ok(mut node) = queries.p4().get_mut(fill_entity) {
            node.width = width;
        }
    }

    let health_label_updates: Vec<(Entity, String)> = queries
        .p5()
        .iter()
        .filter_map(|(marker, children)| {
            let desired = match marker.0 {
                EntityCardSource::LocalPlayer => player_health,
                EntityCardSource::Target => target_health,
            }
            .map(|(current, max)| format!("{current:.0} / {max:.0}"))
            .unwrap_or_default();
            children.iter().find_map(|child| {
                queries.p1().get(child).ok().map(|(_, _)| (child, desired.clone()))
            })
        })
        .collect();

    for (label_entity, desired) in health_label_updates {
        if let Ok((_, mut text)) = queries.p1().get_mut(label_entity) {
            if text.0 != desired {
                text.0 = desired;
            }
        }
    }

    for (marker, mut text) in &mut queries.p1() {
        let desired = match marker.0 {
            EntityCardSource::LocalPlayer if player_entity.is_some() => "Yogg'Sara",
            EntityCardSource::Target => target_name,
            EntityCardSource::LocalPlayer => "",
        };
        if text.0 != desired {
            text.0 = desired.to_owned();
        }
    }
}
