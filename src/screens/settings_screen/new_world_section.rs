use bevy::{ecs::system::SystemParam, input_focus::InputFocus, prelude::*, text::EditableText};

use crate::{
    app::game_state::GameState,
    localization::{Language, UiLocalization},
    ui::{
        button::{compact_control_button, button, primary_button},
        numeric_input::{
            NumericInputEvent, NumericInputFrame, NumericInputSizing, NumericInputState,
            numeric_input_field, sync_numeric_input_view,
        },
        text_input::editable_value,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
    world::{
        NewWorldConfig, WorldLoadMode, WorldSeed, biome::CurrentBiome, dimension::CurrentDimension,
        save_catalog::create_new_world,
    },
};

use super::{
    game_rules_section::TicksPerSecondInputState,
    navigation::{SettingsSection, SettingsSectionSelection},
    spawn_biome_section::{SpawnBiomeDropdownState, spawn_biome_setting},
    world_name_section::{WorldNameFeedback, WorldNameInput, world_name_setting},
    world_settings_section::{GameModeButton, world_settings_section},
};

const RANDOM_SEED_BUTTON_WIDTH: f32 = 190.0;
const SEED_INPUT_MAX_DIGITS: usize = 20;

#[derive(Component)]
pub(super) struct SeedInput;

#[derive(Component)]
pub(super) struct SeedValueText;

#[derive(Component)]
pub(super) struct RandomSeedButton;

type NewWorldGeneralControlInteractions<'w, 's> = Query<
    'w,
    's,
    &'static Interaction,
    (
        Changed<Interaction>,
        Or<(With<RandomSeedButton>, With<GameModeButton>)>,
    ),
>;

pub(super) struct SeedInputKind;
pub(super) type SeedInputState = NumericInputState<SeedInputKind>;

#[derive(Component, Clone, Copy)]
pub(super) enum NewWorldFooterAction {
    Return,
    CreateWorld,
}

#[derive(SystemParam)]
pub(super) struct NewWorldDraft<'w, 's> {
    config: ResMut<'w, NewWorldConfig>,
    seed_input: Res<'w, SeedInputState>,
    ticks_input: Res<'w, TicksPerSecondInputState>,
    spawn_biome_dropdown: Res<'w, SpawnBiomeDropdownState>,
    name_input: Query<'w, 's, (Entity, &'static EditableText), With<WorldNameInput>>,
    focus: ResMut<'w, InputFocus>,
    name_feedback: ResMut<'w, WorldNameFeedback>,
}

impl NewWorldDraft<'_, '_> {
    fn input_editing(&self) -> bool {
        self.seed_input.editing()
            || self.ticks_input.editing()
            || self.spawn_biome_dropdown.input_editing()
    }

    fn commit_seed_input(&mut self) {
        if !self.seed_input.editing() {
            return;
        }
        let Ok(value) = self.seed_input.buffer().parse::<u64>() else {
            return;
        };
        self.config.set_seed(value);
    }
}

pub(super) fn reset_new_world_settings(
    mut config: ResMut<NewWorldConfig>,
    mut selection: ResMut<SettingsSectionSelection>,
    mut seed_input: ResMut<SeedInputState>,
    mut ticks_input: ResMut<TicksPerSecondInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
    mut name_feedback: ResMut<WorldNameFeedback>,
) {
    config.reset();
    selection.selected = SettingsSection::General;
    seed_input.reset();
    ticks_input.reset();
    spawn_biome_dropdown.reset();
    name_feedback.set(String::new());
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
            row_gap: px(8),
            ..default()
        },
        children![
            world_name_setting(config, localization, language),
            seed_setting(config.seed().0, localization, language),
            spawn_biome_setting(localization, language),
            world_settings_section(config.game_mode(), localization, language),
        ],
    )
}

fn seed_setting(seed: u64, localization: &UiLocalization, language: Language) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(8),
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
    footer.spawn(button(
        localization.text(language, "newWorld.return").to_owned(),
        NewWorldFooterAction::Return,
    ));
    footer.spawn(primary_button(
        localization
            .text(language, "newWorld.createWorld")
            .to_owned(),
        NewWorldFooterAction::CreateWorld,
    ));
}

pub(super) fn sync_new_world_input_focus_to_section(
    selection: Res<SettingsSectionSelection>,
    mut seed_input: ResMut<SeedInputState>,
    mut ticks_input: ResMut<TicksPerSecondInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
) {
    if !selection.is_changed() {
        return;
    }

    match selection.selected {
        SettingsSection::General => ticks_input.reset(),
        SettingsSection::GameRules => {
            seed_input.reset();
            spawn_biome_dropdown.close();
        }
        _ => {
            seed_input.reset();
            ticks_input.reset();
            spawn_biome_dropdown.close();
        }
    }
}

pub(super) fn handle_new_world_general_control_focus(
    interactions: NewWorldGeneralControlInteractions,
    mut seed_input: ResMut<SeedInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
) {
    if !interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    seed_input.reset();
    spawn_biome_dropdown.close();
}

pub(super) fn handle_seed_focus(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SeedInput>)>,
    config: Res<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
) {
    let pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    SeedInputState::begin_if_pressed(&mut input, interactions.iter(), config.seed().0);
    if pressed {
        spawn_biome_dropdown.close();
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
        input.reset();
    }
}

pub(super) fn handle_seed_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut config: ResMut<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
    mut editor: Single<(Entity, &mut EditableText), With<SeedInput>>,
) {
    let (entity, editable) = &mut *editor;
    let event = SeedInputState::handle_keyboard(
        &mut input,
        &keys,
        &mut focus,
        *entity,
        editable,
        SEED_INPUT_MAX_DIGITS,
        |next| next.is_empty() || next.parse::<u64>().is_ok(),
    );
    if event == NumericInputEvent::Changed {
        apply_seed_buffer(input.buffer(), &mut config);
    }
}

pub(super) fn handle_new_world_footer(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&Interaction, &NewWorldFooterAction), Changed<Interaction>>,
    mut draft: NewWorldDraft,
    mut transition: ResMut<ScreenTransition>,
) {
    if *game_state.get() != GameState::NewWorld || transition.is_active() {
        return;
    }

    let action = interactions.iter().find_map(|(interaction, action)| {
        (*interaction == Interaction::Pressed).then_some(*action)
    });
    let active_name = draft
        .name_input
        .single()
        .ok()
        .is_some_and(|(entity, _)| draft.focus.get() == Some(entity));
    if keys.just_pressed(KeyCode::Escape) && active_name {
        draft.focus.clear();
        return;
    }
    if matches!(action, Some(NewWorldFooterAction::Return))
        || (keys.just_pressed(KeyCode::Escape) && !draft.input_editing())
    {
        transition.request(ScreenTransitionTarget::game(GameState::StartingScreen));
        return;
    }

    if !matches!(action, Some(NewWorldFooterAction::CreateWorld)) {
        return;
    }

    let requested = match draft.name_input.single() {
        Ok((_, editor)) if !editor.is_composing() => editable_value(editor),
        _ => return,
    };
    draft.commit_seed_input();
    let dimension = CurrentDimension::default();
    let seed = draft.config.seed().0;
    let rules = draft.config.game_rules();
    let name = match create_new_world(
        &requested,
        seed,
        &dimension.id,
        rules.ticks_per_second(),
    ) {
        Ok(name) => name,
        Err(error) => {
            draft.name_feedback.set(error.to_string());
            return;
        }
    };
    draft.config.set_name(name);
    draft.name_feedback.set(String::new());

    commands.insert_resource(dimension);
    commands.insert_resource(CurrentBiome::default());
    commands.insert_resource(WorldSeed(seed));
    commands.insert_resource(rules);
    commands.insert_resource(WorldLoadMode::New);
    transition.request(ScreenTransitionTarget::game(GameState::Loading));
}

pub(super) fn sync_seed_text(
    config: Res<NewWorldConfig>,
    input: Res<SeedInputState>,
    mut labels: Query<&mut EditableText, With<SeedValueText>>,
    mut inputs: Query<&mut BorderColor, With<NumericInputFrame<SeedInput>>>,
) {
    if !config.is_changed() && !input.is_changed() {
        return;
    }

    sync_numeric_input_view(&input, config.seed().0, &mut labels, &mut inputs);
}

fn apply_seed_buffer(buffer: &str, config: &mut ResMut<'_, NewWorldConfig>) {
    if let Ok(value) = buffer.parse::<u64>()
        && config.seed().0 != value
    {
        config.set_seed(value);
    }
}
