use bevy::{ecs::system::SystemParam, input_focus::InputFocus, prelude::*, text::EditableText};

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        numeric_input::{
            NumericInputEvent, NumericInputFrame, NumericInputSizing, NumericInputState,
            numeric_input_field, sync_numeric_input_view,
        },
        selectable, toggle, typography,
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

#[derive(Component, Clone, Copy)]
pub(super) enum BooleanGameRuleToggle {
    SpawnCreatures,
}

#[derive(Component, Clone, Copy)]
pub(super) struct BooleanGameRuleToggleThumb(BooleanGameRuleToggle);

#[derive(SystemParam)]
pub(super) struct TicksPerSecondSettings<'w> {
    game_state: Res<'w, State<GameState>>,
    game_rules: Res<'w, GameRules>,
    new_world: Res<'w, NewWorldConfig>,
}

impl TicksPerSecondSettings<'_> {
    fn current(&self) -> u32 {
        self.current_rules().ticks_per_second()
    }

    fn current_rules(&self) -> GameRules {
        if *self.game_state.get() == GameState::NewWorld {
            self.new_world.game_rules()
        } else {
            *self.game_rules
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

    fn toggle_boolean_rule(&mut self, rule: BooleanGameRuleToggle) {
        if *self.game_state.get() == GameState::NewWorld {
            let current = self.new_world.game_rules();
            match rule {
                BooleanGameRuleToggle::SpawnCreatures => {
                    self.new_world.set_spawn_creatures(!current.spawn_creatures())
                }
            }
            return;
        }

        match rule {
            BooleanGameRuleToggle::SpawnCreatures => {
                let spawn_creatures = !self.game_rules.spawn_creatures();
                self.game_rules.set_spawn_creatures(spawn_creatures);
            }
        }
        self.save.save_game_rules(*self.game_rules);
    }
}

pub(super) fn game_rules_section(
    ticks_per_second: u32,
    rules: GameRules,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(18),
            ..default()
        },
        children![
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
                                localization.text(language, "settings.ticksBySecond").to_owned(),
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
                            button(
                                "−",
                                TicksPerSecondStep::Decrement,
                                px(STEP_BUTTON_SIZE),
                                COMPACT_CONTROL_HEIGHT,
                                ButtonVariant::Normal,
                            ),
                            numeric_input_field(
                                ticks_per_second.to_string(),
                                TicksPerSecondInput,
                                TicksPerSecondValueText,
                                NumericInputSizing::Fixed(INPUT_WIDTH),
                            ),
                            button(
                                "+",
                                TicksPerSecondStep::Increment,
                                px(STEP_BUTTON_SIZE),
                                COMPACT_CONTROL_HEIGHT,
                                ButtonVariant::Normal,
                            ),
                        ],
                    ),
                ],
            ),
            boolean_rule_setting(
                BooleanGameRuleToggle::SpawnCreatures,
                rules.spawn_creatures(),
                "settings.spawnCreatures",
                "settings.spawnCreatures.description",
                localization,
                language,
            ),
        ],
    )
}

fn boolean_rule_setting(
    rule: BooleanGameRuleToggle,
    enabled: bool,
    title_key: &str,
    description_key: &str,
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
                    typography::setting_title(localization.text(language, title_key).to_owned()),
                    typography::caption(
                        localization.text(language, description_key).to_owned(),
                    ),
                ],
            ),
            (
                Button,
                rule,
                toggle::control(enabled),
                children![(BooleanGameRuleToggleThumb(rule), toggle::thumb(enabled))],
            ),
        ],
    )
}

pub(super) fn handle_boolean_game_rule_toggles(
    interactions: Query<(&Interaction, &BooleanGameRuleToggle), Changed<Interaction>>,
    mut settings: TicksPerSecondEditor,
) {
    for (interaction, rule) in &interactions {
        if *interaction == Interaction::Pressed {
            settings.toggle_boolean_rule(*rule);
            break;
        }
    }
}

pub(super) fn sync_boolean_game_rule_toggles(
    settings: TicksPerSecondSettings,
    mut toggles: Query<(
        &BooleanGameRuleToggle,
        Ref<Interaction>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut thumbs: Query<(&BooleanGameRuleToggleThumb, &mut Node)>,
) {
    let changed = settings.inputs_changed();
    let rules = settings.current_rules();

    let enabled = |rule: BooleanGameRuleToggle| match rule {
        BooleanGameRuleToggle::SpawnCreatures => rules.spawn_creatures(),
    };

    for (rule, interaction, background, border) in &mut toggles {
        if !changed && !interaction.is_changed() {
            continue;
        }
        toggle::apply_control_colors(enabled(*rule), *interaction, background, border);
    }

    if changed {
        for (thumb, mut node) in &mut thumbs {
            toggle::apply_thumb_position(enabled(thumb.0), &mut node);
        }
    }
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
        &mut input_state,
        &keys,
        &mut focus,
        *entity,
        editable,
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
    mut inputs: Query<&mut BorderColor, With<NumericInputFrame<TicksPerSecondInput>>>,
) {
    if !settings.inputs_changed() && !input_state.is_changed() {
        return;
    }

    sync_numeric_input_view(&input_state, settings.current(), &mut labels, &mut inputs);
}
