use bevy::{
    prelude::*,
    ui_widgets::observe,
};

use crate::{
    localization::{Language, UiLocalization},
    ui::{
        numeric_input::{
            NumericInputSizing, NumericInputState, numeric_input_field,
        },
        slider, typography,
    },
    world::render_distance::{MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS},
};

use super::render_distance_logic::{apply_render_distance, render_distance_label};

#[derive(Component)]
pub(super) struct RenderDistanceSlider;

#[derive(Component)]
pub(super) struct RenderDistanceSliderThumb;

#[derive(Component)]
pub(super) struct RenderDistanceValueText;

#[derive(Component)]
pub(super) struct RenderDistanceInput;

#[derive(Component)]
pub(super) struct RenderDistanceInputText;

pub(super) struct RenderDistanceInputKind;
pub(super) type RenderDistanceInputState = NumericInputState<RenderDistanceInputKind>;

pub(super) fn graphics_section(
    chunks: i32,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(12),
            ..default()
        },
        children![
            typography::setting_title(
                localization
                    .text(language, "settings.renderDistance")
                    .to_owned(),
            ),
            typography::caption(
                localization
                    .text(language, "settings.renderDistance.description")
                    .to_owned(),
            ),
            (
                typography::muted(render_distance_label(
                    chunks,
                    localization,
                    language,
                )),
                RenderDistanceValueText,
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    ..default()
                },
                children![
                    (
                        slider::slider_track(
                            chunks as f32,
                            MIN_RENDER_DISTANCE_CHUNKS as f32,
                            MAX_RENDER_DISTANCE_CHUNKS as f32,
                            RenderDistanceSlider,
                            RenderDistanceSliderThumb,
                        ),
                        observe(apply_render_distance),
                    ),
                    numeric_input_field(
                        chunks.to_string(),
                        RenderDistanceInput,
                        RenderDistanceInputText,
                        NumericInputSizing::Fixed(slider::SLIDER_NUMBER_INPUT_WIDTH),
                    ),
                ],
            ),
        ],
    )
}
