use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::EditableText,
    ui_widgets::{SliderValue, ValueChange, observe},
};

use crate::{
    localization::{Language, UiLocalization},
    ui::{
        numeric_input::{
            NumericInputEvent, NumericInputFrame, NumericInputSizing, NumericInputState,
            decimal_numeric_input_field, sync_numeric_input_view,
        },
        settings as settings_layout, slider, typography,
    },
    world::{
        BIOME_SIZE_MULTIPLIER_STEP, MAX_BIOME_SIZE_MULTIPLIER,
        MIN_BIOME_SIZE_MULTIPLIER, NewWorldConfig, is_valid_biome_size_multiplier,
        snap_biome_size_multiplier,
    },
};

use super::{
    new_world_section::SeedInputState,
    spawn_biome_section::SpawnBiomeDropdownState,
};

const INPUT_MAX_CHARACTERS: usize = 3;

#[derive(Component)]
pub(super) struct BiomeSizeMultiplierSlider;

#[derive(Component)]
pub(super) struct BiomeSizeMultiplierSliderThumb;

#[derive(Component)]
pub(super) struct BiomeSizeMultiplierInput;

#[derive(Component)]
pub(super) struct BiomeSizeMultiplierInputText;

pub(super) struct BiomeSizeMultiplierInputKind;
pub(super) type BiomeSizeMultiplierInputState =
    NumericInputState<BiomeSizeMultiplierInputKind>;

pub(super) fn biome_size_multiplier_setting(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let value = config.biome_size_multiplier();

    (
        settings_layout::setting_column(),
        children![
            typography::setting_title(
                localization
                    .text(language, "newWorld.biomeSizeMultiplier")
                    .to_owned(),
            ),
            typography::caption(
                localization
                    .text(language, "newWorld.biomeSizeMultiplier.description")
                    .to_owned(),
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
                            value,
                            MIN_BIOME_SIZE_MULTIPLIER,
                            MAX_BIOME_SIZE_MULTIPLIER,
                            BiomeSizeMultiplierSlider,
                            BiomeSizeMultiplierSliderThumb,
                        ),
                        observe(apply_biome_size_multiplier),
                    ),
                    decimal_numeric_input_field(
                        format!("{value:.1}"),
                        BiomeSizeMultiplierInput,
                        BiomeSizeMultiplierInputText,
                        NumericInputSizing::Fixed(slider::SLIDER_NUMBER_INPUT_WIDTH),
                    ),
                ],
            ),
        ],
    )
}

fn apply_biome_size_multiplier(
    value_change: On<ValueChange<f32>>,
    mut commands: Commands,
    mut config: ResMut<NewWorldConfig>,
    mut input_state: ResMut<BiomeSizeMultiplierInputState>,
    mut seed_input: ResMut<SeedInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
) {
    let value = snap_biome_size_multiplier(value_change.value);
    config.set_biome_size_multiplier(value);
    input_state.reset();
    seed_input.reset();
    spawn_biome_dropdown.close();
    commands
        .entity(value_change.source)
        .insert(SliderValue(config.biome_size_multiplier()));
}

pub(super) fn handle_biome_size_multiplier_input(
    interactions: Query<
        &Interaction,
        (Changed<Interaction>, With<BiomeSizeMultiplierInput>),
    >,
    config: Res<NewWorldConfig>,
    mut input_state: ResMut<BiomeSizeMultiplierInputState>,
    mut seed_input: ResMut<SeedInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
) {
    let pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    BiomeSizeMultiplierInputState::begin_if_pressed(
        &mut input_state,
        interactions.iter(),
        format!("{:.1}", config.biome_size_multiplier()),
    );
    if pressed {
        seed_input.reset();
        spawn_biome_dropdown.close();
    }
}

pub(super) fn handle_biome_size_multiplier_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut config: ResMut<NewWorldConfig>,
    mut input_state: ResMut<BiomeSizeMultiplierInputState>,
    mut editor: Single<(Entity, &mut EditableText), With<BiomeSizeMultiplierInput>>,
    mut sliders: Query<&mut SliderValue, With<BiomeSizeMultiplierSlider>>,
) {
    let (entity, editable) = &mut *editor;
    let event = BiomeSizeMultiplierInputState::handle_keyboard(
        &mut input_state,
        &keys,
        &mut focus,
        *entity,
        editable,
        INPUT_MAX_CHARACTERS,
        valid_multiplier_buffer,
    );
    if event != NumericInputEvent::Changed {
        return;
    }

    let Ok(value) = input_state.buffer().parse::<f32>() else {
        return;
    };
    if !is_valid_biome_size_multiplier(value) {
        return;
    }

    config.set_biome_size_multiplier(value);
    let value = config.biome_size_multiplier();
    for mut slider_value in &mut sliders {
        if slider_value.0 != value {
            slider_value.0 = value;
        }
    }
}

pub(super) fn sync_biome_size_multiplier_input(
    config: Res<NewWorldConfig>,
    input_state: Res<BiomeSizeMultiplierInputState>,
    mut labels: Query<&mut EditableText, With<BiomeSizeMultiplierInputText>>,
    mut inputs: Query<
        &mut BorderColor,
        With<NumericInputFrame<BiomeSizeMultiplierInput>>,
    >,
) {
    if !config.is_changed() && !input_state.is_changed() {
        return;
    }

    sync_numeric_input_view(
        &input_state,
        format!("{:.1}", config.biome_size_multiplier()),
        &mut labels,
        &mut inputs,
    );
}

pub(super) fn sync_biome_size_multiplier_slider_thumb(
    sliders: Query<
        &SliderValue,
        (With<BiomeSizeMultiplierSlider>, Changed<SliderValue>),
    >,
    mut thumbs: Query<&mut Node, With<BiomeSizeMultiplierSliderThumb>>,
) {
    let Ok(mut thumb) = thumbs.single_mut() else {
        return;
    };

    for value in &sliders {
        let next_left = percent(
            slider::slider_position(
                value.0,
                MIN_BIOME_SIZE_MULTIPLIER,
                MAX_BIOME_SIZE_MULTIPLIER,
            ) * 100.0,
        );
        if thumb.left != next_left {
            thumb.left = next_left;
        }
    }
}

fn valid_multiplier_buffer(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }

    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some()
        || whole.len() != 1
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_some_and(|digits| {
            digits.len() > 1 || !digits.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return false;
    }

    let Ok(whole_value) = whole.parse::<u8>() else {
        return false;
    };
    if whole_value > MAX_BIOME_SIZE_MULTIPLIER as u8 {
        return false;
    }

    if fraction.is_none() || fraction == Some("") {
        return true;
    }

    value
        .parse::<f32>()
        .is_ok_and(is_valid_biome_size_multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplier_editor_accepts_only_range_and_tenth_step() {
        for value in ["", "0", "0.", "0.5", "1", "1.", "1.0", "4.9", "5", "5.0"] {
            assert!(valid_multiplier_buffer(value), "{value}");
        }
        for value in ["0.4", "5.1", "1.25", "00.5", ".5", "6", "x"] {
            assert!(!valid_multiplier_buffer(value), "{value}");
        }
        assert!((BIOME_SIZE_MULTIPLIER_STEP - 0.1).abs() < f32::EPSILON);
    }
}
