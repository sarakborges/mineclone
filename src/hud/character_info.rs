use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    player::{
        PLAYER_DISPLAY_NAME,
        character_info::CharacterInfoState,
    },
    ui::{surface, theme, typography},
};

use super::player::portrait::{PlayerPreviewImages, PlayerPreviewRenderState};

const CHARACTER_PREVIEW_CARD_WIDTH: f32 = 224.0;
const CHARACTER_PREVIEW_CARD_HEIGHT: f32 = 298.0;
const CHARACTER_PREVIEW_ASPECT_RATIO: f32 = 384.0 / 512.0;
const CHARACTER_PREVIEW_DRAG_SENSITIVITY: f32 = 0.01;

#[derive(Resource, Default)]
struct CharacterPreviewInteraction {
    dragging: bool,
}

#[derive(Component)]
struct CharacterInfoRoot;

#[derive(Component)]
struct CharacterPreviewViewport;

pub(super) struct CharacterInfoHudPlugin;

impl Plugin for CharacterInfoHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterPreviewInteraction>()
            .add_systems(
                OnEnter(CharacterInfoState::Open),
                (request_character_preview, spawn_character_info)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(OnExit(CharacterInfoState::Open), stop_character_preview_drag)
            .add_systems(
                Update,
                rotate_character_preview
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(CharacterInfoState::Open)),
            );
    }
}

fn request_character_preview(
    mut render_state: ResMut<PlayerPreviewRenderState>,
) {
    render_state.request_character();
}

fn spawn_character_info(
    mut commands: Commands,
    preview_images: Res<PlayerPreviewImages>,
) {
    let Some(image) = preview_images.character() else {
        warn!("Character Info opened without a shared character preview target");
        return;
    };

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
                spawn_character_preview_viewport(card, image);
                card.spawn((
                    typography::hud_heading(PLAYER_DISPLAY_NAME),
                    Pickable::IGNORE,
                ));
            });
        });
}

fn spawn_character_preview_viewport(
    parent: &mut ChildSpawnerCommands,
    image: Handle<Image>,
) {
    parent
        .spawn((
            Button,
            CharacterPreviewViewport,
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
                ImageNode::new(image),
                Node {
                    height: percent(100),
                    max_width: percent(100),
                    aspect_ratio: Some(CHARACTER_PREVIEW_ASPECT_RATIO),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

fn stop_character_preview_drag(
    mut interaction: ResMut<CharacterPreviewInteraction>,
) {
    interaction.dragging = false;
}

fn rotate_character_preview(
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    viewports: Query<&Interaction, With<CharacterPreviewViewport>>,
    mut interaction: ResMut<CharacterPreviewInteraction>,
    mut render_state: ResMut<PlayerPreviewRenderState>,
) {
    if mouse.just_released(MouseButton::Left) {
        interaction.dragging = false;
    }

    if mouse.pressed(MouseButton::Left)
        && viewports
            .iter()
            .any(|state| *state == Interaction::Pressed)
    {
        interaction.dragging = true;
    }

    let delta = mouse_motion.read().map(|motion| motion.delta).sum::<Vec2>();
    if !interaction.dragging || !mouse.pressed(MouseButton::Left) || delta.x == 0.0 {
        return;
    }

    render_state.rotate_character(delta.x * CHARACTER_PREVIEW_DRAG_SENSITIVITY);
}
