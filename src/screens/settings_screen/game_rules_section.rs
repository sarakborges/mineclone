use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    ui::{text_input::select_all_pressed, theme, typography},
    world::{InMemoryWorldSave, NewWorldConfig, game_rules::GameRules},
};

const CONTROL_HEIGHT: f32 = 44.0;
const STEP_BUTTON_SIZE: f32 = 44.0;
const INPUT_WIDTH: f32 = 180.0;

#[derive(Component, Clone, Copy)]
pub(crate) enum TicksPerSecondStep {
    Decrement,
    Increment,
}

#[derive(Component)]
pub(crate) struct TicksPerSecondInput;

#[derive(Component)]
pub(crate) struct TicksPerSecondValueText;

#[derive(Resource, Default)]
pub(crate) struct TicksPerSecondInputState {
    editing: bool,
    replace_on_next_digit: bool,
    buffer: String,
}

impl TicksPerSecondInputState {
    pub(crate) fn editing(&self) -> bool {
        self.editing
    }
}

pub(crate) fn game_rules_section(
    ticks_per_second: u32,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(24),
            ..default()
        },
        children![
            (
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    ..default()
                },
                children![
                    typography::setting_title(
                        localization
                            .text(language, "settings.ticksBySecond")
                            .to_owned(),
                    ),
                    typography::caption(
                        localization
                            .text(language, "settings.ticksBySecond.description")
                            .to_owned(),
                    ),
                ],
            ),
            (
                Node {
                    flex_shrink: 0.0,
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
            padding: UiRect::axes(px(14), px(0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.045, 0.035, 0.09, 0.88)),
        BorderColor::all(input_border(false)),
        children![(
            typography::button_label(value.to_string()),
            TicksPerSecondValueText,
        )],
    )
}

pub(crate) fn handle_ticks_step_buttons(
    interactions: Query<(&Interaction, &TicksPerSecondStep), Changed<Interaction>>,
    game_state: Res<State<GameState>>,
    mut game_rules: ResMut<GameRules>,
    mut new_world: ResMut<NewWorldConfig>,
    mut save: ResMut<InMemoryWorldSave>,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    for (interaction, step) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let current = current_ticks_per_second(*game_state.get(), &game_rules, &new_world);
        let next = match step {
            TicksPerSecondStep::Decrement => current.saturating_sub(1).max(1),
            TicksPerSecondStep::Increment => current.saturating_add(1),
        };

        set_ticks_per_second(
            next,
            *game_state.get(),
            &mut game_rules,
            &mut new_world,
            &mut save,
        );
        *input_state = TicksPerSecondInputState::default();
    }
}

pub(crate) fn handle_ticks_input(
    interactions: Query<&Interaction, (Changed<Interaction>, With<TicksPerSecondInput>)>,
    game_state: Res<State<GameState>>,
    game_rules: Res<GameRules>,
    new_world: Res<NewWorldConfig>,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        input_state.editing = true;
        input_state.replace_on_next_digit = false;
        input_state.buffer =
            current_ticks_per_second(*game_state.get(), &game_rules, &new_world).to_string();
    }
}

pub(crate) fn handle_ticks_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    mut game_rules: ResMut<GameRules>,
    mut new_world: ResMut<NewWorldConfig>,
    mut save: ResMut<InMemoryWorldSave>,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    if !input_state.editing {
        return;
    }

    if select_all_pressed(&keys) {
        input_state.replace_on_next_digit = true;
        return;
    }

    if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || keys.just_pressed(KeyCode::Escape)
    {
        *input_state = TicksPerSecondInputState::default();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::NumpadBackspace) {
        if input_state.replace_on_next_digit {
            input_state.buffer.clear();
            input_state.replace_on_next_digit = false;
        } else {
            input_state.buffer.pop();
        }
        apply_input_buffer(
            &input_state.buffer,
            *game_state.get(),
            &mut game_rules,
            &mut new_world,
            &mut save,
        );
        return;
    }

    for (key, digit) in digit_keys() {
        if !keys.just_pressed(key) {
            continue;
        }

        if input_state.replace_on_next_digit {
            input_state.buffer.clear();
            input_state.replace_on_next_digit = false;
        }

        if input_state.buffer.len() < 10 {
            input_state.buffer.push(digit);
            apply_input_buffer(
                &input_state.buffer,
                *game_state.get(),
                &mut game_rules,
                &mut new_world,
                &mut save,
            );
        }
        break;
    }
}

pub(crate) fn sync_ticks_per_second_text(
    game_state: Res<State<GameState>>,
    game_rules: Res<GameRules>,
    new_world: Res<NewWorldConfig>,
    input_state: Res<TicksPerSecondInputState>,
    mut labels: Query<&mut Text, With<TicksPerSecondValueText>>,
    mut inputs: Query<&mut BorderColor, With<TicksPerSecondInput>>,
) {
    let value = if input_state.editing {
        format!("{}|", input_state.buffer)
    } else {
        current_ticks_per_second(*game_state.get(), &game_rules, &new_world).to_string()
    };

    for mut label in &mut labels {
        if label.0 != value {
            label.0 = value.clone();
        }
    }

    for mut border in &mut inputs {
        *border = BorderColor::all(input_border(input_state.editing));
    }
}

fn current_ticks_per_second(
    game_state: GameState,
    game_rules: &GameRules,
    new_world: &NewWorldConfig,
) -> u32 {
    if game_state == GameState::NewWorld {
        new_world.game_rules().ticks_per_second()
    } else {
        game_rules.ticks_per_second()
    }
}

fn digit_keys() -> [(KeyCode, char); 20] {
    [
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
        (KeyCode::Numpad0, '0'),
        (KeyCode::Numpad1, '1'),
        (KeyCode::Numpad2, '2'),
        (KeyCode::Numpad3, '3'),
        (KeyCode::Numpad4, '4'),
        (KeyCode::Numpad5, '5'),
        (KeyCode::Numpad6, '6'),
        (KeyCode::Numpad7, '7'),
        (KeyCode::Numpad8, '8'),
        (KeyCode::Numpad9, '9'),
    ]
}

fn apply_input_buffer(
    buffer: &str,
    game_state: GameState,
    game_rules: &mut GameRules,
    new_world: &mut NewWorldConfig,
    save: &mut InMemoryWorldSave,
) {
    let Ok(value) = buffer.parse::<u32>() else {
        return;
    };
    if value == 0 {
        return;
    }

    set_ticks_per_second(value, game_state, game_rules, new_world, save);
}

fn set_ticks_per_second(
    value: u32,
    game_state: GameState,
    game_rules: &mut GameRules,
    new_world: &mut NewWorldConfig,
    save: &mut InMemoryWorldSave,
) {
    if game_state == GameState::NewWorld {
        new_world.set_ticks_per_second(value);
        return;
    }

    game_rules.set_ticks_per_second(value);
    save.save_game_rules(*game_rules);
}

fn input_border(editing: bool) -> Color {
    if editing {
        theme::TEXT_PRIMARY.with_alpha(0.92)
    } else {
        Color::srgba(0.43, 0.36, 0.68, 0.72)
    }
}
