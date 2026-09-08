use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    ui_widgets::{observe, Slider, SliderRange, SliderThumb, SliderValue, TrackClick, ValueChange},
};

use crate::{
    app::settings_state::SettingsState,
    ui::{button::menu_button, theme, typography},
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::{
        RenderDistanceSettings, MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS,
    },
};

const SLIDER_WIDTH: f32 = 360.0;
const SLIDER_THUMB_SIZE: f32 = 16.0;
const BACKDROP_DOWNSCALE: u32 = 4;
const BACKDROP_BLUR_SIGMA: f32 = 4.5;

pub struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .add_systems(OnEnter(SettingsState::Open), capture_settings_backdrop)
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

fn capture_settings_backdrop(mut commands: Commands) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(build_settings_menu_from_capture);
}

fn build_settings_menu_from_capture(
    capture: On<ScreenshotCaptured>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    render_distance: Res<RenderDistanceSettings>,
    settings_state: Res<State<SettingsState>>,
) {
    if *settings_state.get() != SettingsState::Open {
        return;
    }

    let backdrop = capture
        .image
        .clone()
        .try_into_dynamic()
        .ok()
        .map(|source| {
            let width = (source.width() / BACKDROP_DOWNSCALE).max(1);
            let height = (source.height() / BACKDROP_DOWNSCALE).max(1);
            let softened = source
                .resize_exact(width, height, ::image::imageops::FilterType::Triangle)
                .blur(BACKDROP_BLUR_SIGMA);

            images.add(Image::from_dynamic(
                softened,
                true,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            ))
        });

    spawn_settings_menu(&mut commands, backdrop, render_distance.chunks());
}

fn spawn_settings_menu(
    commands: &mut Commands,
    backdrop: Option<Handle<Image>>,
    chunks: i32,
) {
    commands
        .spawn((
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
            BackgroundColor(if backdrop.is_some() {
                Color::NONE
            } else {
                theme::OVERLAY
            }),
        ))
        .with_children(|root| {
            if let Some(backdrop) = backdrop {
                root.spawn((
                    ImageNode::new(backdrop),
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
                    Pickable::IGNORE,
                ));

                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        right: px(0),
                        top: px(0),
                        bottom: px(0),
                        ..default()
                    },
                    BackgroundColor(theme::OVERLAY),
                    Pickable::IGNORE,
                ));
            }

            root.spawn((
                Node {
                    width: px(560),
                    padding: UiRect::all(px(34)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(20),
                    border_radius: BorderRadius::all(px(10)),
                    ..default()
                },
                BackgroundColor(theme::FROSTED_SURFACE),
                theme::frosted_surface_gradient(),
                BoxShadow(vec![ShadowStyle {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.42),
                    x_offset: px(0),
                    y_offset: px(12),
                    spread_radius: px(0),
                    blur_radius: px(30),
                }]),
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
                    typography::muted(render_distance_label(chunks)),
                    RenderDistanceValueText,
                ));
                panel.spawn(render_distance_slider(chunks));
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

fn handle_back_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SettingsBackButton>)>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            next_settings_state.set(SettingsState::Closed);
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
