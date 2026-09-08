use bevy::{
    prelude::*,
    ui_widgets::{observe, Slider, SliderRange, SliderThumb, SliderValue, TrackClick, ValueChange},
};

use crate::{
    app::settings_state::SettingsState,
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::{
        RenderDistanceSettings, MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS,
    },
};

const OVERLAY_COLOR: Color = Color::srgba(0.02, 0.025, 0.04, 0.96);
const PANEL_COLOR: Color = Color::srgb(0.08, 0.095, 0.12);
const BUTTON_COLOR: Color = Color::srgb(0.12, 0.14, 0.18);
const BUTTON_HOVER_COLOR: Color = Color::srgb(0.18, 0.21, 0.27);
const BUTTON_PRESSED_COLOR: Color = Color::srgb(0.09, 0.11, 0.14);
const BORDER_COLOR: Color = Color::srgb(0.28, 0.32, 0.4);
const TEXT_COLOR: Color = Color::srgb(0.92, 0.94, 0.97);
const MUTED_TEXT_COLOR: Color = Color::srgb(0.66, 0.7, 0.78);
const SLIDER_TRACK_COLOR: Color = Color::srgb(0.12, 0.14, 0.18);
const SLIDER_THUMB_COLOR: Color = Color::srgb(0.76, 0.82, 0.94);
const SLIDER_WIDTH: f32 = 360.0;
const SLIDER_THUMB_SIZE: f32 = 16.0;

pub struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .add_systems(OnEnter(SettingsState::Open), spawn_settings_menu)
            .add_systems(
                Update,
                (
                    handle_back_button,
                    close_settings_with_escape,
                    sync_render_distance_text,
                    sync_slider_thumb,
                )
                    .run_if(in_state(SettingsState::Open)),
            );
    }
}

#[derive(Component)]
struct SettingsBackButton;

#[derive(Component)]
struct RenderDistanceSlider;

#[derive(Component)]
struct RenderDistanceSliderThumb;

#[derive(Component)]
struct RenderDistanceValueText;

fn spawn_settings_menu(mut commands: Commands, render_distance: Res<RenderDistanceSettings>) {
    let chunks = render_distance.chunks();

    commands.spawn((
        DespawnOnExit(SettingsState::Open),
        Node {
            width: percent(100),
            height: percent(100),
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(OVERLAY_COLOR),
        children![(
            Node {
                width: px(520),
                padding: UiRect::all(px(32)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(20),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(PANEL_COLOR),
            BorderColor::all(BORDER_COLOR),
            children![
                (
                    Text::new("SETTINGS"),
                    TextFont {
                        font_size: FontSize::Px(42.0),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                ),
                (
                    Text::new("Render Distance"),
                    TextFont {
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ),
                (
                    Text::new(render_distance_label(chunks)),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(MUTED_TEXT_COLOR),
                    RenderDistanceValueText,
                ),
                render_distance_slider(chunks),
                (
                    Text::new("Applied when a world is loaded."),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(MUTED_TEXT_COLOR),
                    Node {
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                ),
                back_button(),
            ],
        )],
    ));
}

fn render_distance_slider(chunks: i32) -> impl Bundle {
    let initial_position = slider_position(chunks as f32);

    (
        RenderDistanceSlider,
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(chunks as f32),
        SliderRange::new(
            MIN_RENDER_DISTANCE_CHUNKS as f32,
            MAX_RENDER_DISTANCE_CHUNKS as f32,
        ),
        Node {
            width: px(SLIDER_WIDTH),
            height: px(32),
            position_type: PositionType::Relative,
            ..default()
        },
        observe(
            |value_change: On<ValueChange<f32>>,
             mut commands: Commands,
             mut render_distance: ResMut<RenderDistanceSettings>| {
                let chunks = value_change.value.round() as i32;
                render_distance.set_chunks(chunks);
                commands
                    .entity(value_change.source)
                    .insert(SliderValue(render_distance.chunks() as f32));
            },
        ),
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    top: px(13),
                    height: px(6),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK_COLOR),
            ),
            (
                SliderThumb,
                RenderDistanceSliderThumb,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(SLIDER_THUMB_SIZE),
                    height: px(SLIDER_THUMB_SIZE),
                    left: percent(initial_position * 100.0),
                    top: px(8),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(SLIDER_THUMB_COLOR),
            ),
        ],
    )
}

fn back_button() -> impl Bundle {
    (
        Button,
        SettingsBackButton,
        Node {
            width: px(280),
            height: px(56),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(BUTTON_COLOR),
        BorderColor::all(BORDER_COLOR),
        children![(
            Text::new("Back"),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(TEXT_COLOR),
        )],
    )
}

fn handle_back_button(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<SettingsBackButton>),
    >,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
) {
    for (interaction, mut background) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BUTTON_PRESSED_COLOR.into();
                next_settings_state.set(SettingsState::Closed);
            }
            Interaction::Hovered => *background = BUTTON_HOVER_COLOR.into(),
            Interaction::None => *background = BUTTON_COLOR.into(),
        }
    }
}

fn close_settings_with_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_settings_state.set(SettingsState::Closed);
    }
}

fn sync_render_distance_text(
    render_distance: Res<RenderDistanceSettings>,
    mut labels: Query<&mut Text, With<RenderDistanceValueText>>,
) {
    if !render_distance.is_changed() {
        return;
    }

    for mut label in &mut labels {
        **label = render_distance_label(render_distance.chunks());
    }
}

fn sync_slider_thumb(
    sliders: Query<&SliderValue, (With<RenderDistanceSlider>, Changed<SliderValue>)>,
    mut thumbs: Query<&mut Node, With<RenderDistanceSliderThumb>>,
) {
    let Ok(mut thumb) = thumbs.single_mut() else {
        return;
    };

    for value in &sliders {
        thumb.left = percent(slider_position(value.0) * 100.0);
    }
}

fn slider_position(value: f32) -> f32 {
    let min = MIN_RENDER_DISTANCE_CHUNKS as f32;
    let max = MAX_RENDER_DISTANCE_CHUNKS as f32;
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn render_distance_label(chunks: i32) -> String {
    format!("{chunks} chunks ({} blocks)", chunks * CHUNK_SIZE as i32)
}
