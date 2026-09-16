use bevy::{ecs::system::SystemParam, input_focus::InputFocus, prelude::*, text::EditableText};

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    ui::{
        button::compact_control_button,
        numeric_input::{
            NumericInputEvent, NumericInputSizing, NumericInputState, numeric_input_field,
            sync_numeric_input_view,
        },
        typography,
    },
    world::{InMemoryWorldSave, NewWorldConfig, game_rules::GameRules},
};

const STEP_BUTTON_SIZE: f32 = 44.0;
const INPUT_WIDTH: f32 = 180.0;
const TICKS_INPUT_MAX_DIGITS: usize = 10;

#[derive(Component, Clone, Copy)]
pub(super) enum TicksPerSecondStep {
    Decrement,
    Increment,
}

#[derive(Component)]
pub(super) struct TicksPerSecondInput;

#[derive(Component)]
pub(super) struct TicksPerSecondValueText;

pub(super) struct TicksPerSecondInputKind;
pub(super) type TicksPerSecondInputState = NumericInputState<TicksPerSecondInputKind>;

#[derive(SystemParam)]
pub(super) struct TicksPerSecondSettings<'w> {
    game_state: Res<'w, State<GameState>>,
    game_rules: Res<'w, GameRules>,
    new_world: Res<'w, NewWorldConfig>,
}

impl TicksPerSecondSettings<'_> {
    fn current(&self) -> u32 {
        if *self.game_state.get() == GameState::NewWorld {
            self.new_world.game_rules().ticks_per_second()
        } else {
            self.game_rules.ticks_per_second()
        }
    }

    fn inputs_changed(&self) -> bool {
        self.game_state.is_changed() || self.game_rules.is_changed() || self.new_world.is_changed()
    }
}

#[derive(SystemParam)]
pub(super) struct TicksPerSecondEditor<'w> {
    game_state: Res<'w, State<GameState>>,
    game_rules: ResMut<'w, GameRules>,
    new_world: ResMut<'w, NewWorldConfig>,
    save: ResMut<'w, InMemoryWorldSave>,
}

impl TicksPerSecondEditor<'_> {
    fn current(&self) -> u32 {
        if *self.game_state.get() == GameState::NewWorld {
            self.new_world.game_rules().ticks_per_second()
        } else {
            self.game_rules.ticks_per_second()
        }
    }

    fn set(&mut self, value: u32) {
        if self.current() == value {
            return;
        }
        if *self.game_state.get() == GameState::NewWorld {
            self.new_world.set_ticks_per_second(value);
            return;
        }

        self.game_rules.set_ticks_per_second(value);
        self.save.save_game_rules(*self.game_rules);
    }

    fn apply_buffer(&mut self, buffer: &str) {
        let Ok(value) = buffer.parse::<u32>() else {
            return;
        };
        if value == 0 {
            return;
        }

        self.set(value);
    }
}

pub(super) fn game_rules_section(
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
                    compact_control_button("−", TicksPerSecondStep::Decrement, STEP_BUTTON_SIZE,),
                    numeric_input_field(
                        ticks_per_second.to_string(),
                        TicksPerSecondInput,
                        TicksPerSecondValueText,
                        NumericInputSizing::Fixed(INPUT_WIDTH),
                    ),
                    compact_control_button("+", TicksPerSecondStep::Increment, STEP_BUTTON_SIZE,),
                ],
            ),
        ],
    )
}

pub(super) fn handle_ticks_step_buttons(
    interactions: Query<(&Interaction, &TicksPerSecondStep), Changed<Interaction>>,
    mut settings: TicksPerSecondEditor,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    for (interaction, step) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let current = settings.current();
        let next = match step {
            TicksPerSecondStep::Decrement => current.saturating_sub(1).max(1),
            TicksPerSecondStep::Increment => current.saturating_add(1),
        };

        settings.set(next);
        input_state.reset();
    }
}

pub(super) fn handle_ticks_input(
    interactions: Query<&Interaction, (Changed<Interaction>, With<TicksPerSecondInput>)>,
    settings: TicksPerSecondSettings,
    mut input_state: ResMut<TicksPerSecondInputState>,
) {
    TicksPerSecondInputState::begin_if_pressed(
        &mut input_state,
        interactions.iter(),
        settings.current(),
    );
}

pub(super) fn handle_ticks_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut settings: TicksPerSecondEditor,
    mut input_state: ResMut<TicksPerSecondInputState>,
    mut editor: Single<(Entity, &mut EditableText), With<TicksPerSecondInput>>,
) {
    let (entity, editable) = &mut *editor;
    let event = TicksPerSecondInputState::handle_keyboard(
        &mut input_state, &keys, &mut focus, *entity, editable,
        TICKS_INPUT_MAX_DIGITS,
        |next| next.is_empty() || next.parse::<u32>().is_ok(),
    );
    if event == NumericInputEvent::Changed {
        settings.apply_buffer(input_state.buffer());
    }
}

pub(super) fn sync_ticks_per_second_text(
    settings: TicksPerSecondSettings,
    input_state: Res<TicksPerSecondInputState>,
    mut labels: Query<&mut EditableText, With<TicksPerSecondValueText>>,
    mut inputs: Query<&mut BorderColor, With<TicksPerSecondInput>>,
) {
    if !settings.inputs_changed() && !input_state.is_changed() {
        return;
    }

    sync_numeric_input_view(&input_state, settings.current(), &mut labels, &mut inputs);
}
