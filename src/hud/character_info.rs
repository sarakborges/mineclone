use bevy::{input::mouse::MouseMotion, prelude::*};

use crate::{
    app::game_state::GameState,
    entity::EntityHealth,
    player::{
        PLAYER_DISPLAY_NAME,
        camera::GameplayCamera,
        character_info::CharacterInfoState,
    },
    ui::{selectable, surface, theme, typography},
};

use super::{
    inventory::{
        CharacterInfoInventoryRoot, CharacterInfoInventorySpawn, INVENTORY_SLOT_GAP,
        INVENTORY_SLOT_SIZE,
    },
    player::portrait::{CharacterInfoPreviewViewport, CharacterPreviewOrbit},
};

const CHARACTER_INFO_PANEL_WIDTH: f32 = 480.0;
const CHARACTER_INFO_DETAILS_WIDTH: f32 = 202.0;
const CHARACTER_INFO_PANEL_GAP: f32 = 24.0;
const CHARACTER_PREVIEW_CARD_WIDTH: f32 = 224.0;
const CHARACTER_PREVIEW_CARD_HEIGHT: f32 = 298.0;
const CHARACTER_PREVIEW_IMAGE_WIDTH: f32 = 216.0;
const CHARACTER_PREVIEW_IMAGE_HEIGHT: f32 = 288.0;
const CHARACTER_PREVIEW_DRAG_SENSITIVITY: f32 = 0.01;
const CHARACTER_NAME_HEALTH_GAP: f32 = 14.0;
const CHARACTER_SECTION_LABEL_GAP: f32 = 8.0;
const CHARACTER_HEALTH_EQUIPMENT_GAP: f32 = 18.0;
const CHARACTER_HEALTH_BAR_HEIGHT: f32 = 22.0;
const CHARACTER_HEALTH_FILL_COLOR: Color = Color::srgba(0.78, 0.16, 0.25, 0.94);
const EQUIPMENT_LABEL_GAP: f32 = 10.0;

#[derive(Resource, Default)]
struct CharacterPreviewInteraction {
    dragging: bool,
}

#[derive(Component)]
struct CharacterInfoRoot;

#[derive(Component)]
struct CharacterInfoHealthFill;

#[derive(Component)]
struct CharacterInfoHealthLabel;

pub(super) struct CharacterInfoHudPlugin;

impl Plugin for CharacterInfoHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterPreviewInteraction>()
            .init_resource::<CharacterPreviewOrbit>()
            .add_systems(
                OnEnter(CharacterInfoState::Open),
                spawn_character_info.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(CharacterInfoState::Open),
                stop_character_preview_drag,
            )
            .add_systems(
                Update,
                (rotate_character_preview, sync_character_info_health)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(CharacterInfoState::Open)),
            );
    }
}

fn spawn_character_info(
    mut commands: Commands,
    mut inventory: CharacterInfoInventorySpawn,
) {
    commands
        .spawn((
            CharacterInfoRoot,
            CharacterInfoInventoryRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(CHARACTER_INFO_PANEL_GAP),
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
            DespawnOnExit(CharacterInfoState::Open),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            spawn_character_info_panel(root);
            inventory.spawn(root);
        });
}

fn spawn_character_info_panel(root: &mut ChildSpawnerCommands) {
    root.spawn((
        surface::hud_container(Node {
            width: px(CHARACTER_INFO_PANEL_WIDTH),
            padding: UiRect::all(px(18)),
            border: UiRect::all(px(1)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexStart,
            column_gap: px(18),
            ..default()
        }),
        Pickable::IGNORE,
    ))
    .with_children(|card| {
        spawn_character_preview_viewport(card);
        card.spawn((
            Node {
                width: px(CHARACTER_INFO_DETAILS_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|details| {
            details.spawn((
                typography::hud_heading(PLAYER_DISPLAY_NAME),
                Pickable::IGNORE,
            ));
            spawn_character_health_bar(details);
            spawn_equipment_table(details);
        });
    });
}

fn spawn_character_preview_viewport(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: px(CHARACTER_PREVIEW_CARD_WIDTH),
                height: px(CHARACTER_PREVIEW_CARD_HEIGHT),
                flex_shrink: 0.0,
                padding: UiRect::all(px(2)),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::SURFACE_INSET),
            BorderColor::all(theme::BORDER),
        ))
        .with_children(|frame| {
            frame.spawn((
                Button,
                CharacterInfoPreviewViewport,
                Node {
                    width: px(CHARACTER_PREVIEW_IMAGE_WIDTH),
                    height: px(CHARACTER_PREVIEW_IMAGE_HEIGHT),
                    ..default()
                },
            ));
        });
}

fn spawn_character_health_bar(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: percent(100),
                margin: UiRect::top(px(CHARACTER_NAME_HEALTH_GAP)),
                flex_direction: FlexDirection::Column,
                row_gap: px(CHARACTER_SECTION_LABEL_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|section| {
            section.spawn((
                typography::hud_subheading("Health"),
                Pickable::IGNORE,
            ));
            section
                .spawn((
                    Node {
                        position_type: PositionType::Relative,
                        width: percent(100),
                        height: px(CHARACTER_HEALTH_BAR_HEIGHT),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(theme::SLIDER_TRACK),
                    BorderColor::all(selectable::BORDER_COLOR),
                    Pickable::IGNORE,
                ))
                .with_children(|health| {
                    health.spawn((
                        CharacterInfoHealthFill,
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            top: px(0),
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(CHARACTER_HEALTH_FILL_COLOR),
                        Pickable::IGNORE,
                    ));
                    health
                        .spawn((
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
                                CharacterInfoHealthLabel,
                                typography::inventory_category(""),
                                typography::tooltip_shadow(),
                                TextLayout::justify(Justify::Center),
                                Pickable::IGNORE,
                            ));
                        });
                });
        });
}

fn spawn_equipment_table(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: percent(100),
                margin: UiRect::top(px(CHARACTER_HEALTH_EQUIPMENT_GAP)),
                flex_direction: FlexDirection::Column,
                row_gap: px(CHARACTER_SECTION_LABEL_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|section| {
            section.spawn((
                typography::hud_subheading("Armor"),
                Pickable::IGNORE,
            ));
            section
                .spawn((
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(INVENTORY_SLOT_GAP),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|table| {
                    for label in [
                        "No helm equiped",
                        "No breastplate equiped",
                        "No leggings equiped",
                        "No boots equiped",
                    ] {
                        spawn_equipment_row(table, label);
                    }
                });
        });
}

fn spawn_equipment_row(parent: &mut ChildSpawnerCommands, label: &'static str) {
    let (background, border) = selectable::static_colors(false);
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(INVENTORY_SLOT_SIZE),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(EQUIPMENT_LABEL_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(INVENTORY_SLOT_SIZE),
                    height: px(INVENTORY_SLOT_SIZE),
                    min_width: px(INVENTORY_SLOT_SIZE),
                    min_height: px(INVENTORY_SLOT_SIZE),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(background),
                BorderColor::all(border),
                Pickable::IGNORE,
            ));
            row.spawn((typography::caption(label), Pickable::IGNORE));
        });
}

fn sync_character_info_health(
    player: Query<&EntityHealth, With<GameplayCamera>>,
    mut fill: Single<&mut Node, With<CharacterInfoHealthFill>>,
    mut label: Single<&mut Text, With<CharacterInfoHealthLabel>>,
) {
    let health = player
        .iter()
        .next()
        .map(|health| (health.current(), health.max()));
    let fraction = health
        .map(|(current, max)| (current / max).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    fill.width = percent(fraction * 100.0);

    let desired = health
        .map(|(current, max)| format!("{current:.0} / {max:.0}"))
        .unwrap_or_default();
    if label.0 != desired {
        label.0 = desired;
    }
}

fn rotate_character_preview(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    interactions: Query<&Interaction, With<CharacterInfoPreviewViewport>>,
    mut interaction: ResMut<CharacterPreviewInteraction>,
    mut orbit: ResMut<CharacterPreviewOrbit>,
) {
    if mouse.just_pressed(MouseButton::Left)
        && interactions
            .iter()
            .any(|state| *state == Interaction::Pressed)
    {
        interaction.dragging = true;
    }
    if mouse.just_released(MouseButton::Left) {
        interaction.dragging = false;
    }

    let delta_x: f32 = motion.read().map(|event| event.delta.x).sum();
    if interaction.dragging && delta_x != 0.0 {
        orbit.rotate(delta_x * CHARACTER_PREVIEW_DRAG_SENSITIVITY);
    }
}

fn stop_character_preview_drag(mut interaction: ResMut<CharacterPreviewInteraction>) {
    interaction.dragging = false;
}
