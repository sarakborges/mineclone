use bevy::{input::mouse::MouseMotion, prelude::*};

use crate::{
    app::game_state::GameState,
    player::{
        PLAYER_DISPLAY_NAME,
        character_info::CharacterInfoState,
    },
    ui::{surface, theme, typography},
};

use super::player::portrait::{CharacterInfoPreviewViewport, CharacterPreviewOrbit};

const CHARACTER_PREVIEW_CARD_WIDTH: f32 = 224.0;
const CHARACTER_PREVIEW_CARD_HEIGHT: f32 = 298.0;
const CHARACTER_PREVIEW_IMAGE_WIDTH: f32 = 216.0;
const CHARACTER_PREVIEW_IMAGE_HEIGHT: f32 = 288.0;
const CHARACTER_PREVIEW_DRAG_SENSITIVITY: f32 = 0.01;

#[derive(Resource, Default)]
struct CharacterPreviewInteraction {
    dragging: bool,
}

#[derive(Component)]
struct CharacterInfoRoot;

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
                rotate_character_preview
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(CharacterInfoState::Open)),
            );
    }
}

fn spawn_character_info(mut commands: Commands) {

    commands
        .spawn((
            CharacterInfoRoot,
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
            DespawnOnExit(CharacterInfoState::Open),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                surface::hud_container(Node {
                    width: px(460),
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
                    typography::hud_heading(PLAYER_DISPLAY_NAME),
                    Pickable::IGNORE,
                ));
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
