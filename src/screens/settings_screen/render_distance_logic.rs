use bevy::{
    prelude::*,
    ui_widgets::{SliderValue, ValueChange},
};

use crate::{
    localization::{ActiveLanguage, Language, UiLocalization},
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::{
        MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS, RenderDistanceSettings,
    },
};

use super::render_distance_section::{
    RenderDistanceSlider, RenderDistanceSliderThumb, RenderDistanceValueText,
};

pub(super) fn apply_render_distance(
    value_change: On<ValueChange<f32>>,
    mut commands: Commands,
    mut render_distance: ResMut<RenderDistanceSettings>,
) {
    let chunks = value_change.value.round() as i32;
    render_distance.set_chunks(chunks);
    commands
        .entity(value_change.source)
        .insert(SliderValue(render_distance.chunks() as f32));
}

pub(super) fn sync_render_distance_text(
    render_distance: Res<RenderDistanceSettings>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut labels: Query<&mut Text, With<RenderDistanceValueText>>,
) {
    if !render_distance.is_changed() && !language.is_changed() {
        return;
    }

    let next = render_distance_label(render_distance.chunks(), &localization, language.get());
    for mut label in &mut labels {
        if label.0 != next {
            label.0 = next.clone();
        }
    }
}

pub(super) fn sync_slider_thumb(
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

pub(super) fn slider_position(value: f32) -> f32 {
    let min = MIN_RENDER_DISTANCE_CHUNKS as f32;
    let max = MAX_RENDER_DISTANCE_CHUNKS as f32;
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

pub(super) fn render_distance_label(
    chunks: i32,
    localization: &UiLocalization,
    language: Language,
) -> String {
    localization
        .text(language, "settings.renderDistance.value")
        .replace("{chunks}", &chunks.to_string())
        .replace("{blocks}", &(chunks * CHUNK_SIZE as i32).to_string())
}
