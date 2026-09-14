use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    ui::{
        button::{compact_control_button, menu_button},
        numeric_input::{
            NumericInputEvent, NumericInputSizing, NumericInputState, numeric_input_field,
            sync_numeric_input_view,
        },
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

const RANDOM_SEED_BUTTON_WIDTH: f32 = 190.0;
const SEED_INPUT_MAX_DIGITS: usize = 20;

#[derive(Component)]
pub(super) struct SeedInput;

#[derive(Component)]
pub(super) struct SeedValueText;

#[derive(Component)]
pub(super) struct RandomSeedButton;

pub(super) struct SeedInputKind;
pub(super) type SeedInputState = NumericInputState<SeedInputKind>;

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
    seed_input.reset();
    ticks_input.reset();
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
                    numeric_input_field(
                        seed.to_string(),
                        SeedInput,
                        SeedValueText,
                        NumericInputSizing::Flexible,
                    ),
                    compact_control_button(
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
    input.begin_if_pressed(interactions.iter(), config.seed().0);
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
        input.reset();
    }
}

pub(super) fn handle_seed_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
) {
    let event = input.handle_keyboard(&keys, SEED_INPUT_MAX_DIGITS, |next| {
        next.parse::<u64>().is_ok()
    });
    if event == NumericInputEvent::Changed {
        apply_seed_buffer(input.buffer(), &mut config);
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

    let input_editing = seed_input.editing() || ticks_input.editing();
    if matches!(action, Some(NewWorldFooterAction::Return))
        || (keys.just_pressed(KeyCode::Escape) && !input_editing)
    {
        transition.request(ScreenTransitionTarget::game(GameState::StartingScreen));
        return;
    }

    if !matches!(action, Some(NewWorldFooterAction::CreateWorld)) {
        return;
    }

    if seed_input.editing() {
        apply_seed_buffer(seed_input.buffer(), &mut config);
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
    sync_numeric_input_view(&input, config.seed().0, &mut labels, &mut inputs);
}

fn apply_seed_buffer(buffer: &str, config: &mut NewWorldConfig) {
    if let Ok(value) = buffer.parse::<u64>() {
        config.set_seed(value);
    }
}
