use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    ui::{
        button::menu_button,
        text_input::select_all_pressed,
        theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
    world::{
        NewWorldConfig, WorldLoadMode, WorldSeed, biome::CurrentBiome,
        dimension::CurrentDimension,
    },
};

use super::{
    game_rules_section::TicksPerSecondInputState,
    navigation::{SettingsSection, SettingsSectionSelection},
    world_settings_section::world_settings_section,
};

const CONTROL_HEIGHT: f32 = 44.0;
const RANDOM_SEED_BUTTON_WIDTH: f32 = 190.0;

#[derive(Component)]
struct SeedInput;

#[derive(Component)]
struct SeedValueText;

#[derive(Component)]
struct RandomSeedButton;

#[derive(Resource, Default)]
pub(super) struct SeedInputState {
    editing: bool,
    replace_on_next_digit: bool,
    buffer: String,
}

#[derive(Component, Clone, Copy)]
pub(super) enum NewWorldFooterAction {
    Return,
    CreateWorld,
}

pub(super) fn reset_new_world_settings(
    mut config: ResMut<NewWorldConfig>,
    mut selection: ResMut<SettingsSectionSelection>,
    mut seed_input: ResMut<SeedInputState>,
    mut ticks_input: ResMut<TicksPerSecondInputState>,
) {
    config.reset();
    selection.selected = SettingsSection::General;
    *seed_input = SeedInputState::default();
    *ticks_input = TicksPerSecondInputState::default();
}

pub(super) fn new_world_general_section(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(24),
            ..default()
        },
        children![
            seed_setting(config.seed().0, localization, language),
            world_settings_section(config.game_mode(), localization, language),
        ],
    )
}

fn seed_setting(
    seed: u64,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(10),
            ..default()
        },
        children![
            typography::setting_title(localization.text(language, "newWorld.seed").to_owned()),
            typography::caption(
                localization
                    .text(language, "newWorld.seed.description")
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
                    seed_input(seed),
                    compact_button(
                        localization
                            .text(language, "newWorld.randomSeed")
                            .to_owned(),
                        RandomSeedButton,
                        RANDOM_SEED_BUTTON_WIDTH,
                    ),
                ],
            ),
        ],
    )
}

fn seed_input(seed: u64) -> impl Bundle {
    (
        Button,
        SeedInput,
        Node {
            flex_grow: 1.0,
            min_width: px(0),
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
            typography::button_label(seed.to_string()),
            SeedValueText,
        )],
    )
}

fn compact_button<A: Component>(label: impl Into<String>, action: A, width: f32) -> impl Bundle {
    (
        Button,
        action,
        Node {
            width: px(width),
            height: px(CONTROL_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.20, 0.14, 0.38, 0.72)),
        children![typography::button_label(label)],
    )
}

pub(super) fn spawn_new_world_footer(
    footer: &mut ChildSpawnerCommands,
    localization: &UiLocalization,
    language: Language,
) {
    footer.spawn(menu_button(
        localization.text(language, "newWorld.return").to_owned(),
        NewWorldFooterAction::Return,
    ));
    footer.spawn(menu_button(
        localization
            .text(language, "newWorld.createWorld")
            .to_owned(),
        NewWorldFooterAction::CreateWorld,
    ));
}

pub(super) fn handle_seed_focus(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SeedInput>)>,
    config: Res<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        input.editing = true;
        input.replace_on_next_digit = false;
        input.buffer = config.seed().0.to_string();
    }
}

pub(super) fn handle_random_seed(
    interactions: Query<&Interaction, (Changed<Interaction>, With<RandomSeedButton>)>,
    mut config: ResMut<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        config.set_seed(WorldSeed::fresh().0);
        *input = SeedInputState::default();
    }
}

pub(super) fn handle_seed_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
) {
    if !input.editing {
        return;
    }

    if select_all_pressed(&keys) {
        input.replace_on_next_digit = true;
        return;
    }

    if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || keys.just_pressed(KeyCode::Escape)
    {
        *input = SeedInputState::default();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::NumpadBackspace) {
        if input.replace_on_next_digit {
            input.buffer.clear();
            input.replace_on_next_digit = false;
        } else {
            input.buffer.pop();
        }
        apply_seed_buffer(&input.buffer, &mut config);
        return;
    }

    for (key, digit) in digit_keys() {
        if !keys.just_pressed(key) {
            continue;
        }

        let mut next = if input.replace_on_next_digit {
            String::new()
        } else {
            input.buffer.clone()
        };
        next.push(digit);

        if next.len() <= 20
            && let Ok(value) = next.parse::<u64>()
        {
            input.buffer = next;
            input.replace_on_next_digit = false;
            config.set_seed(value);
        }
        break;
    }
}

pub(super) fn handle_new_world_footer(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&Interaction, &NewWorldFooterAction), Changed<Interaction>>,
    mut config: ResMut<NewWorldConfig>,
    seed_input: Res<SeedInputState>,
    ticks_input: Res<TicksPerSecondInputState>,
    mut transition: ResMut<ScreenTransition>,
) {
    if *game_state.get() != GameState::NewWorld {
        return;
    }

    let action = interactions.iter().find_map(|(interaction, action)| {
        (*interaction == Interaction::Pressed).then_some(*action)
    });

    let input_editing = seed_input.editing || ticks_input.editing();
    if matches!(action, Some(NewWorldFooterAction::Return))
        || (keys.just_pressed(KeyCode::Escape) && !input_editing)
    {
        transition.request(ScreenTransitionTarget::game(GameState::StartingScreen));
        return;
    }

    if !matches!(action, Some(NewWorldFooterAction::CreateWorld)) {
        return;
    }

    if seed_input.editing {
        apply_seed_buffer(&seed_input.buffer, &mut config);
    }

    commands.insert_resource(CurrentDimension::default());
    commands.insert_resource(CurrentBiome::default());
    commands.insert_resource(WorldSeed(config.seed().0));
    commands.insert_resource(config.game_rules());
    commands.insert_resource(WorldLoadMode::New);
    transition.request(ScreenTransitionTarget::game(GameState::Loading));
}

pub(super) fn sync_seed_text(
    config: Res<NewWorldConfig>,
    input: Res<SeedInputState>,
    mut labels: Query<&mut Text, With<SeedValueText>>,
    mut inputs: Query<&mut BorderColor, With<SeedInput>>,
) {
    let value = if input.editing {
        format!("{}|", input.buffer)
    } else {
        config.seed().0.to_string()
    };

    for mut label in &mut labels {
        if label.0 != value {
            label.0 = value.clone();
        }
    }

    for mut border in &mut inputs {
        *border = BorderColor::all(input_border(input.editing));
    }
}

fn apply_seed_buffer(buffer: &str, config: &mut NewWorldConfig) {
    if let Ok(value) = buffer.parse::<u64>() {
        config.set_seed(value);
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

fn input_border(editing: bool) -> Color {
    if editing {
        theme::TEXT_PRIMARY.with_alpha(0.92)
    } else {
        Color::srgba(0.43, 0.36, 0.68, 0.72)
    }
}
