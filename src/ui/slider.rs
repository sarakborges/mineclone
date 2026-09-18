use bevy::{
    prelude::*,
    ui_widgets::{Slider, SliderRange, SliderThumb, SliderValue, TrackClick},
};

use super::theme;

pub(crate) const SLIDER_NUMBER_INPUT_WIDTH: f32 = 112.0;
const SLIDER_THUMB_SIZE: f32 = 16.0;

pub(crate) fn slider_track<S: Component, T: Component>(
    value: f32,
    min: f32,
    max: f32,
    slider_marker: S,
    thumb_marker: T,
) -> impl Bundle {
    (
        slider_marker,
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(value),
        SliderRange::new(min, max),
        Node {
            width: Val::Auto,
            min_width: px(0),
            flex_grow: 1.0,
            height: px(32),
            position_type: PositionType::Relative,
            ..default()
        },
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    top: px(13),
                    height: px(6),
                    ..default()
                },
                BackgroundColor(theme::SLIDER_TRACK),
            ),
            (
                SliderThumb,
                thumb_marker,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(SLIDER_THUMB_SIZE),
                    height: px(SLIDER_THUMB_SIZE),
                    left: percent(slider_position(value, min, max) * 100.0),
                    top: px(8),
                    ..default()
                },
                BackgroundColor(theme::SLIDER_THUMB),
            ),
        ],
    )
}

pub(crate) fn slider_position(value: f32, min: f32, max: f32) -> f32 {
    assert!(max > min, "slider range must have positive width");
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_position_clamps_to_track() {
        assert_eq!(slider_position(0.0, 0.5, 5.0), 0.0);
        assert_eq!(slider_position(5.5, 0.5, 5.0), 1.0);
        assert!((slider_position(2.75, 0.5, 5.0) - 0.5).abs() < f32::EPSILON);
    }
}
