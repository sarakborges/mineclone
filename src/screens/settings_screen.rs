use bevy::{
    prelude::*,
    ui_widgets::{observe, Slider, SliderRange, SliderThumb, SliderValue, TrackClick, ValueChange},
};

use crate::{
    app::settings_state::SettingsState,
    ui::{
        button::menu_button,
        cosmic_background::{self, STAR_FIELD},
        surface,
        theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::{
        RenderDistanceSettings, MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS,
    },
};

const CONTENT_WIDTH: f32 = 760.0;
const HEADER_HEIGHT: f32 = 116.0;
const FOOTER_HEIGHT: f32 = 104.0;
const SLIDER_THUMB_SIZE: f32 = 16.0;

pub struct SettingsScreenPlugin;

impl Plugin for SettingsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .add_systems(OnEnter(SettingsState::Open), spawn_settings_screen)
            .add_systems(
                Update,
                (
                    handle_close_requests,
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

fn spawn_settings_screen(mut commands: Commands, render_distance: Res<RenderDistanceSettings>) {
    commands
        .spawn((
            DespawnOnExit(SettingsState::Open),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
            GlobalZIndex(500),
        ))
        .with_children(|root| {
            for &(left, top, size, phase, speed, red, green, blue, base_alpha) in STAR_FIELD {
                root.spawn(cosmic_background::star(
                    left, top, size, phase, speed, red, green, blue, base_alpha,
                ));
            }

            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(HEADER_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn(typography::title("SETTINGS"));
            });

            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(HEADER_HEIGHT),
                bottom: px(FOOTER_HEIGHT),
                left: px(0),
                right: px(0),
                padding: UiRect::axes(px(32), px(24)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    width: px(CONTENT_WIDTH),
                    max_width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(24),
                    ..default()
                })
                .with_children(|sections| {
                    sections
                        .spawn(surface::settings_section())
                        .with_children(|section| {
                            section.spawn(typography::heading("Render Distance"));
                            section.spawn((
                                typography::muted(render_distance_label(render_distance.chunks())),
                                RenderDistanceValueText,
                            ));
                            section.spawn(render_distance_slider(render_distance.chunks()));
                            section.spawn(typography::caption(
                                "Controls how far terrain is generated and rendered around the player.",
                            ));
                        });
                });
            });

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                bottom: px(0),
                height: px(FOOTER_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|footer| {
                footer.spawn(menu_button("Back", SettingsBackButton));
            });
        });
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
            width: percent(100),
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
                BackgroundColor(theme::SLIDER_TRACK),
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
                BackgroundColor(theme::SLIDER_THUMB),
                BoxShadow(vec![ShadowStyle {
                    color: theme::CYAN_GLOW,
                    x_offset: px(0),
                    y_offset: px(0),
                    spread_radius: px(0),
                    blur_radius: px(12),
                }]),
            ),
        ],
    )
}

fn handle_close_requests(
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<SettingsBackButton>)>,
    mut transition: ResMut<ScreenTransition>,
) {
    let back_pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);

    if back_pressed || keys.just_pressed(KeyCode::Escape) {
        transition.request(ScreenTransitionTarget::settings(SettingsState::Closed));
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
