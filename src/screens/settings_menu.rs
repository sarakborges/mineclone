use bevy::{
    prelude::*,
    ui_widgets::{observe, Slider, SliderRange, SliderThumb, SliderValue, TrackClick, ValueChange},
};

use crate::{
    app::settings_state::SettingsState,
    ui::{button::menu_button, surface, theme, typography},
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::{
        RenderDistanceSettings, MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS,
    },
};

const SLIDER_WIDTH: f32 = 360.0;
const SLIDER_THUMB_SIZE: f32 = 16.0;
const MENU_TRANSITION_SECONDS: f32 = 0.18;
const MENU_START_SCALE: f32 = 0.965;
const MENU_START_Y: f32 = 14.0;

pub struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .add_systems(OnEnter(SettingsState::Open), spawn_settings_menu)
            .add_systems(
                Update,
                (
                    handle_close_requests,
                    animate_settings_transition,
                    sync_render_distance_text,
                    sync_slider_thumb,
                )
                    .run_if(in_state(SettingsState::Open)),
            );
    }
}

#[derive(Component)]
struct SettingsMenuRoot;

#[derive(Component)]
struct SettingsMenuPanel;

#[derive(Component)]
struct SettingsMenuTransition {
    progress: f32,
    closing: bool,
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
    commands
        .spawn((
            SettingsMenuRoot,
            SettingsMenuTransition {
                progress: 0.0,
                closing: false,
            },
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
            BackgroundColor(theme::OVERLAY),
        ))
        .with_children(|root| {
            root.spawn((
                surface::modal_panel(),
                SettingsMenuPanel,
                menu_panel_transform(0.0),
            ))
            .with_children(|panel| {
                panel.spawn((
                    typography::title("SETTINGS"),
                    Node {
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                ));
                panel.spawn(typography::label("Render Distance"));
                panel.spawn((
                    typography::muted(render_distance_label(render_distance.chunks())),
                    RenderDistanceValueText,
                ));
                panel.spawn(render_distance_slider(render_distance.chunks()));
                panel.spawn((
                    typography::caption("Applied when a world is loaded."),
                    Node {
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                ));
                panel.spawn(menu_button("Back", SettingsBackButton));
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
    mut transitions: Query<&mut SettingsMenuTransition, With<SettingsMenuRoot>>,
) {
    let back_pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);

    if !back_pressed && !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    for mut transition in &mut transitions {
        transition.closing = true;
    }
}

fn animate_settings_transition(
    time: Res<Time>,
    mut transitions: Query<&mut SettingsMenuTransition, With<SettingsMenuRoot>>,
    mut panels: Query<&mut UiTransform, With<SettingsMenuPanel>>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
) {
    let step = time.delta_secs() / MENU_TRANSITION_SECONDS;
    let Ok(mut panel_transform) = panels.single_mut() else {
        return;
    };

    for mut transition in &mut transitions {
        if transition.closing {
            transition.progress = (transition.progress - step).max(0.0);
        } else {
            transition.progress = (transition.progress + step).min(1.0);
        }

        *panel_transform = menu_panel_transform(ease_out_cubic(transition.progress));

        if transition.closing && transition.progress <= 0.0 {
            next_settings_state.set(SettingsState::Closed);
        }
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

fn menu_panel_transform(progress: f32) -> UiTransform {
    UiTransform {
        translation: Val2::px(0.0, MENU_START_Y * (1.0 - progress)),
        scale: Vec2::splat(MENU_START_SCALE + (1.0 - MENU_START_SCALE) * progress),
        ..default()
    }
}

fn ease_out_cubic(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(3)
}

fn slider_position(value: f32) -> f32 {
    let min = MIN_RENDER_DISTANCE_CHUNKS as f32;
    let max = MAX_RENDER_DISTANCE_CHUNKS as f32;
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn render_distance_label(chunks: i32) -> String {
    format!("{chunks} chunks ({} blocks)", chunks * CHUNK_SIZE as i32)
}
