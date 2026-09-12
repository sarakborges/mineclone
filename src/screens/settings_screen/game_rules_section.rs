use bevy::prelude::*;

use crate::{
    ui::{surface, theme, typography},
    world::{InMemoryWorldSave, game_rules::GameRules},
};

const CONTROL_HEIGHT: f32 = 44.0;
const STEP_BUTTON_SIZE: f32 = 44.0;
const INPUT_WIDTH: f32 = 124.0;

#[derive(Component, Clone, Copy)]
pub(super) enum TicksPerSecondStep {
    Decrement,
    Increment,
}

#[derive(Component)]
pub(super) struct TicksPerSecondInput;

#[derive(Component)]
pub(super) struct TicksPerSecondValueText;

#[derive(Resource, Default)]
pub(super) struct TicksPerSecondInputState {
    editing: bool,
    replace_on_next_digit: bool,
    buffer: String,
}

pub(super) fn game_rules_section(ticks_per_second: u32) -> impl Bundle {
    (
        surface::settings_section(),
        children![
            typography::heading("Game Rules"),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    ..default()
                },
                children![
                    typography::muted("Ticks by Second"),
                    (
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(10),
                            ..default()
                        },
                        children![
                            step_button("−", TicksPerSecondStep::Decrement),
                            ticks_input(ticks_per_second),
                            step_button("+", TicksPerSecondStep::Increment),
                        ],
                    ),
                    typography::caption(
                        "Controls how many simulation ticks the world advances per real second.",
                    ),
                ],
            ),
        ],
    )
}

fn step_button(label: &'static str, step: TicksPerSecondStep) -> impl Bundle {
    (
        Button,
        step,
        Node {
            width: px(STEP_BUTTON_SIZE),
            height: px(STEP_BUTTON_SIZE),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.20, 0.14, 0.38, 0.72)),
        children![typography::button_label(label)],
    )
}

fn ticks_input(value: u32) -> impl Bundle {
    (
        Button,
        TicksPerSecondInput,
        Node {
            width: px(INPUT_WIDTH),
            height: px(CONTROL_HEIGHT),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.045, 0.035, 0.09, 0.88)),
        BorderColor::all(Color::srgba(0.43, 0.36, 0.68, 0.72)),
        children![(
            typography::button_label(value.to_string()),
            TicksPerSecondValueText,
        )],
    )
}

pub(super) fn handle_ticks_step_buttons(
    interactions: Query<(&Interaction, &TicksPerSecondStep), Changed<Interaction>>,
    mut game_rules: ResMut<GameRules>,
    mut save: ResMut<InMemoryWorldSave>,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    for (interaction, step) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let current = game_rules.ticks_per_second();
        let next = match step {
            TicksPerSecondStep::Decrement => current.saturating_sub(1).max(1),
            TicksPerSecondStep::Increment => current.saturating_add(1),
        };

        set_ticks_per_second(next, &mut game_rules, &mut save);
        input_state.editing = false;
        input_state.replace_on_next_digit = false;
        input_state.buffer.clear();
    }
}

pub(super) fn handle_ticks_input(
    interactions: Query<&Interaction, (Changed<Interaction>, With<TicksPerSecondInput>)>,
    game_rules: Res<GameRules>,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        input_state.editing = true;
        input_state.replace_on_next_digit = true;
        input_state.buffer = game_rules.ticks_per_second().to_string();
    }
}

pub(super) fn handle_ticks_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut game_rules: ResMut<GameRules>,
    mut save: ResMut<InMemoryWorldSave>,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    if !input_state.editing {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Escape) {
        input_state.editing = false;
        input_state.replace_on_next_digit = false;
        input_state.buffer.clear();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        if input_state.replace_on_next_digit {
            input_state.buffer.clear();
            input_state.replace_on_next_digit = false;
        } else {
            input_state.buffer.pop();
        }
        apply_input_buffer(&input_state.buffer, &mut game_rules, &mut save);
        return;
    }

    for (key, digit) in [
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
    ] {
        if !keys.just_pressed(key) {
            continue;
        }

        if input_state.replace_on_next_digit {
            input_state.buffer.clear();
            input_state.replace_on_next_digit = false;
        }

        if input_state.buffer.len() < 10 {
            input_state.buffer.push(digit);
            apply_input_buffer(&input_state.buffer, &mut game_rules, &mut save);
        }
        break;
    }
}

pub(super) fn sync_ticks_per_second_text(
    game_rules: Res<GameRules>,
    input_state: Res<TicksPerSecondInputState>,
    mut labels: Query<&mut Text, With<TicksPerSecondValueText>>,
) {
    let value = if input_state.editing {
        input_state.buffer.clone()
    } else {
        game_rules.ticks_per_second().to_string()
    };

    for mut label in &mut labels {
        if label.0 != value {
            label.0 = value.clone();
        }
    }
}

fn apply_input_buffer(
    buffer: &str,
    game_rules: &mut GameRules,
    save: &mut InMemoryWorldSave,
) {
    let Ok(value) = buffer.parse::<u32>() else {
        return;
    };
    if value == 0 {
        return;
    }

    set_ticks_per_second(value, game_rules, save);
}

fn set_ticks_per_second(
    value: u32,
    game_rules: &mut GameRules,
    save: &mut InMemoryWorldSave,
) {
    game_rules.set_ticks_per_second(value);
    save.save_game_rules(*game_rules);
}
