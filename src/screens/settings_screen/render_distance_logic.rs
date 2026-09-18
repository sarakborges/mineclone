use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::EditableText,
    ui_widgets::{SliderValue, ValueChange},
};

use crate::{
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        numeric_input::{NumericInputEvent, NumericInputFrame, sync_numeric_input_view},
        slider,
    },
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::{
        MAX_RENDER_DISTANCE_CHUNKS, MIN_RENDER_DISTANCE_CHUNKS, RenderDistanceSettings,
    },
};

use super::render_distance_section::{
    RenderDistanceInput, RenderDistanceInputState, RenderDistanceInputText,
    RenderDistanceSlider, RenderDistanceSliderThumb, RenderDistanceValueText,
};

pub(super) fn apply_render_distance(
    value_change: On<ValueChange<f32>>,
    mut commands: Commands,
    mut render_distance: ResMut<RenderDistanceSettings>,
    mut input_state: ResMut<RenderDistanceInputState>,
) {
    let chunks = value_change.value.round() as i32;
    render_distance.set_chunks(chunks);
    input_state.reset();
    commands
        .entity(value_change.source)
        .insert(SliderValue(render_distance.chunks() as f32));
}

pub(super) fn handle_render_distance_input(
    interactions: Query<&Interaction, (Changed<Interaction>, With<RenderDistanceInput>)>,
    render_distance: Res<RenderDistanceSettings>,
    mut input_state: ResMut<RenderDistanceInputState>,
) {
    RenderDistanceInputState::begin_if_pressed(
        &mut input_state,
        interactions.iter(),
        render_distance.chunks(),
    );
}

pub(super) fn handle_render_distance_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut render_distance: ResMut<RenderDistanceSettings>,
    mut input_state: ResMut<RenderDistanceInputState>,
    mut editor: Single<(Entity, &mut EditableText), With<RenderDistanceInput>>,
    mut sliders: Query<&mut SliderValue, With<RenderDistanceSlider>>,
) {
    let (entity, editable) = &mut *editor;
    let event = RenderDistanceInputState::handle_keyboard(
        &mut input_state,
        &keys,
        &mut focus,
        *entity,
        editable,
        MAX_RENDER_DISTANCE_CHUNKS.to_string().len(),
        valid_render_distance_buffer,
    );
    if event != NumericInputEvent::Changed {
        return;
    }

    let Ok(chunks) = input_state.buffer().parse::<i32>() else {
        return;
    };
    if !(MIN_RENDER_DISTANCE_CHUNKS..=MAX_RENDER_DISTANCE_CHUNKS).contains(&chunks) {
        return;
    }

    render_distance.set_chunks(chunks);
    for mut slider_value in &mut sliders {
        let next = render_distance.chunks() as f32;
        if slider_value.0 != next {
            slider_value.0 = next;
        }
    }
}

pub(super) fn sync_render_distance_input(
    render_distance: Res<RenderDistanceSettings>,
    input_state: Res<RenderDistanceInputState>,
    mut labels: Query<&mut EditableText, With<RenderDistanceInputText>>,
    mut inputs: Query<&mut BorderColor, With<NumericInputFrame<RenderDistanceInput>>>,
) {
    if !render_distance.is_changed() && !input_state.is_changed() {
        return;
    }

    sync_numeric_input_view(
        &input_state,
        render_distance.chunks(),
        &mut labels,
        &mut inputs,
    );
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

pub(super) fn sync_render_distance_slider_thumb(
    sliders: Query<&SliderValue, (With<RenderDistanceSlider>, Changed<SliderValue>)>,
    mut thumbs: Query<&mut Node, With<RenderDistanceSliderThumb>>,
) {
    let Ok(mut thumb) = thumbs.single_mut() else {
        return;
    };

    for value in &sliders {
        let next_left = percent(
            slider::slider_position(
                value.0,
                MIN_RENDER_DISTANCE_CHUNKS as f32,
                MAX_RENDER_DISTANCE_CHUNKS as f32,
            ) * 100.0,
        );
        if thumb.left != next_left {
            thumb.left = next_left;
        }
    }
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

fn valid_render_distance_buffer(value: &str) -> bool {
    value.is_empty()
        || value
            .parse::<i32>()
            .is_ok_and(|chunks| chunks <= MAX_RENDER_DISTANCE_CHUNKS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_distance_input_rejects_values_above_slider_range() {
        assert!(valid_render_distance_buffer(""));
        assert!(valid_render_distance_buffer("3"));
        assert!(valid_render_distance_buffer("4"));
        assert!(valid_render_distance_buffer("24"));
        assert!(!valid_render_distance_buffer("25"));
    }
}
