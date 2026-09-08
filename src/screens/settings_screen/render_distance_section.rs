use bevy::{
    prelude::*,
    ui_widgets::{observe, Slider, SliderRange, SliderThumb, SliderValue, TrackClick},
};

use crate::{
    ui::{surface, theme, typography},
    world::render_distance::{MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS},
};

use super::render_distance_logic::{
    apply_render_distance, render_distance_label, slider_position,
};

const SLIDER_THUMB_SIZE: f32 = 16.0;

#[derive(Component)]
pub(super) struct RenderDistanceSlider;

#[derive(Component)]
pub(super) struct RenderDistanceSliderThumb;

#[derive(Component)]
pub(super) struct RenderDistanceValueText;

pub(super) fn render_distance_section(chunks: i32) -> impl Bundle {
    (
        surface::settings_section(),
        children![
            typography::heading("Render Distance"),
            (
                typography::muted(render_distance_label(chunks)),
                RenderDistanceValueText,
            ),
            render_distance_slider(chunks),
            typography::caption(
                "Controls how far terrain is generated and rendered around the player.",
            ),
        ],
    )
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
        observe(apply_render_distance),
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
