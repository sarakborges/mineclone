use bevy::{ecs::system::SystemParam, input_focus::InputFocus, prelude::*, text::EditableText};

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        button::{
            button, ButtonVariant, COMPACT_CONTROL_HEIGHT, MENU_BUTTON_HEIGHT, MENU_BUTTON_WIDTH,
        },
        numeric_input::{
            NumericInputEvent, NumericInputFrame, NumericInputSizing, NumericInputState,
            numeric_input_field, sync_numeric_input_view,
        },
        selectable,
        settings as settings_layout,
        theme,
        text_input::editable_value,
        toggle,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
    world::{
        NewWorldConfig, WorldGenerationMode, WorldLoadMode, WorldSeed,
        biome::CurrentBiome, dimension::CurrentDimension, save_catalog::create_new_world,
    },
};

use super::{
    biome_size_multiplier_section::{
        BiomeSizeMultiplierInputState, biome_size_multiplier_setting,
    },
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

#[derive(Component, Clone, Copy)]
pub(super) struct WorldGenerationModeButton(pub(super) WorldGenerationMode);

#[derive(Component)]
pub(super) struct SpawnStructuresToggle;

#[derive(Component)]
pub(super) struct SpawnStructuresToggleThumb;

#[derive(Component)]
pub(super) struct SingleBiomeToggle;

#[derive(Component)]
pub(super) struct SingleBiomeToggleThumb;

#[derive(Component, Clone, Copy)]
pub(super) enum WorldGenerationFeatureToggle {
    Caves,
    Rivers,
    Lakes,
    Oceans,
}

#[derive(Component, Clone, Copy)]
pub(super) struct WorldGenerationFeatureToggleThumb(WorldGenerationFeatureToggle);

type NewWorldGeneralControlInteractions<'w, 's> = Query<
    'w,
    's,
    &'static Interaction,
    (
        Changed<Interaction>,
        Or<(
            With<RandomSeedButton>,
            With<GameModeButton>,
            With<WorldGenerationModeButton>,
            With<SpawnStructuresToggle>,
            With<SingleBiomeToggle>,
            With<WorldGenerationFeatureToggle>,
        )>,
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
pub(super) struct NewWorldFooterInput<'w, 's> {
    game_state: Res<'w, State<GameState>>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    interactions: Query<
        'w,
        's,
        (&'static Interaction, &'static NewWorldFooterAction),
        Changed<Interaction>,
    >,
}

impl NewWorldFooterInput<'_, '_> {
    fn is_new_world(&self) -> bool {
        *self.game_state.get() == GameState::NewWorld
    }

    fn pressed_action(&self) -> Option<NewWorldFooterAction> {
        self.interactions.iter().find_map(|(interaction, action)| {
            (*interaction == Interaction::Pressed).then_some(*action)
        })
    }

    fn escape_pressed(&self) -> bool {
        self.keys.just_pressed(KeyCode::Escape)
    }
}

#[derive(SystemParam)]
pub(super) struct NewWorldDraft<'w, 's> {
    config: ResMut<'w, NewWorldConfig>,
    seed_input: Res<'w, SeedInputState>,
    ticks_input: Res<'w, TicksPerSecondInputState>,
    biome_size_input: Res<'w, BiomeSizeMultiplierInputState>,
    spawn_biome_dropdown: ResMut<'w, SpawnBiomeDropdownState>,
    name_input: Query<'w, 's, (Entity, &'static EditableText), With<WorldNameInput>>,
    focus: ResMut<'w, InputFocus>,
    name_feedback: ResMut<'w, WorldNameFeedback>,
}

impl NewWorldDraft<'_, '_> {
    fn input_editing(&self) -> bool {
        self.seed_input.editing()
            || self.ticks_input.editing()
            || self.biome_size_input.editing()
            || self.spawn_biome_dropdown.is_open()
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
    mut biome_size_input: ResMut<BiomeSizeMultiplierInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
    mut name_feedback: ResMut<WorldNameFeedback>,
) {
    config.reset();
    selection.selected = SettingsSection::WorldSettings;
    seed_input.reset();
    ticks_input.reset();
    biome_size_input.reset();
    spawn_biome_dropdown.reset();
    name_feedback.set(String::new());
}

pub(super) fn new_world_settings_section(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        settings_layout::group_column(),
        children![
            world_name_setting(config, localization, language),
            seed_setting(config.seed().0, localization, language),
            world_settings_section(config.game_mode(), localization, language),
        ],
    )
}

pub(super) fn new_world_generation_section(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        settings_layout::group_column(),
        children![
            world_generation_mode_setting(config, localization, language),
            spawn_biome_setting(localization, language),
            biome_size_multiplier_setting(config, localization, language),
            spawn_structures_setting(config, localization, language),
            single_biome_setting(config, localization, language),
            world_generation_feature_setting(
                WorldGenerationFeatureToggle::Caves,
                config,
                "newWorld.spawnCaves",
                "newWorld.spawnCaves.description",
                localization,
                language,
            ),
            world_generation_feature_setting(
                WorldGenerationFeatureToggle::Rivers,
                config,
                "newWorld.spawnRivers",
                "newWorld.spawnRivers.description",
                localization,
                language,
            ),
            world_generation_feature_setting(
                WorldGenerationFeatureToggle::Lakes,
                config,
                "newWorld.spawnLakes",
                "newWorld.spawnLakes.description",
                localization,
                language,
            ),
            world_generation_feature_setting(
                WorldGenerationFeatureToggle::Oceans,
                config,
                "newWorld.spawnOceans",
                "newWorld.spawnOceans.description",
                localization,
                language,
            ),
        ],
    )
}

fn world_generation_mode_setting(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let mode = config.world_generation().mode();
    (
        settings_layout::setting_column(),
        children![
            typography::setting_title(
                localization.text(language, "newWorld.worldType").to_owned()
            ),
            typography::caption(
                localization.text(language, "newWorld.worldType.description").to_owned()
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    ..default()
                },
                children![
                    world_generation_mode_button(
                        localization.text(language, "newWorld.worldType.normal").to_owned(),
                        WorldGenerationMode::Normal,
                        mode,
                    ),
                    world_generation_mode_button(
                        localization.text(language, "newWorld.worldType.flat").to_owned(),
                        WorldGenerationMode::Flat,
                        mode,
                    ),
                    world_generation_mode_button(
                        localization.text(language, "newWorld.worldType.void").to_owned(),
                        WorldGenerationMode::Void,
                        mode,
                    ),
                ],
            ),
        ],
    )
}

fn world_generation_mode_button(
    label: String,
    mode: WorldGenerationMode,
    selected: WorldGenerationMode,
) -> impl Bundle {
    button(
        label,
        WorldGenerationModeButton(mode),
        Val::Auto,
        COMPACT_CONTROL_HEIGHT,
        ButtonVariant::from_active(mode == selected),
    )
}

fn spawn_structures_setting(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let enabled = config.world_generation().spawn_structures();
    (
        worldgen_toggle_row(),
        children![
            worldgen_setting_copy(
                localization.text(language, "newWorld.spawnStructures"),
                localization.text(language, "newWorld.spawnStructures.description"),
            ),
            (
                Button,
                SpawnStructuresToggle,
                toggle::control(enabled),
                children![(SpawnStructuresToggleThumb, toggle::thumb(enabled))],
            ),
        ],
    )
}

fn single_biome_setting(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let enabled = config.world_generation().single_biome();
    (
        worldgen_toggle_row(),
        children![
            worldgen_setting_copy(
                localization.text(language, "newWorld.singleBiome"),
                localization.text(language, "newWorld.singleBiome.description"),
            ),
            (
                Button,
                SingleBiomeToggle,
                toggle::control(enabled),
                children![(SingleBiomeToggleThumb, toggle::thumb(enabled))],
            ),
        ],
    )
}

fn world_generation_feature_setting(
    feature: WorldGenerationFeatureToggle,
    config: &NewWorldConfig,
    title_key: &str,
    description_key: &str,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let settings = config.world_generation();
    let enabled = match feature {
        WorldGenerationFeatureToggle::Caves => settings.spawn_caves(),
        WorldGenerationFeatureToggle::Rivers => settings.spawn_rivers(),
        WorldGenerationFeatureToggle::Lakes => settings.spawn_lakes(),
        WorldGenerationFeatureToggle::Oceans => settings.spawn_oceans(),
    };
    let disabled = settings.mode() == WorldGenerationMode::Void;
    let (background, border) = if disabled {
        (theme::SURFACE_INSET, theme::BORDER)
    } else {
        toggle::colors(enabled, Interaction::None)
    };

    (
        worldgen_toggle_row(),
        children![
            worldgen_setting_copy(
                localization.text(language, title_key),
                localization.text(language, description_key),
            ),
            (
                Button,
                feature,
                Node {
                    width: px(toggle::WIDTH),
                    height: px(toggle::HEIGHT),
                    flex_shrink: 0.0,
                    position_type: PositionType::Relative,
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(background),
                BorderColor::all(border),
                children![(
                    WorldGenerationFeatureToggleThumb(feature),
                    toggle::thumb(enabled),
                )],
            ),
        ],
    )
}

fn worldgen_toggle_row() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: px(18),
        ..default()
    }
}

fn worldgen_setting_copy(title: impl Into<String>, description: impl Into<String>) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            ..default()
        },
        children![
            typography::setting_title(title),
            typography::caption(description),
        ],
    )
}

fn seed_setting(seed: u64, localization: &UiLocalization, language: Language) -> impl Bundle {
    (
        settings_layout::setting_column(),
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
                    button(
                        localization
                            .text(language, "newWorld.randomSeed")
                            .to_owned(),
                        RandomSeedButton,
                        px(RANDOM_SEED_BUTTON_WIDTH),
                        COMPACT_CONTROL_HEIGHT,
                        ButtonVariant::Normal,
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
        px(MENU_BUTTON_WIDTH),
        MENU_BUTTON_HEIGHT,
        ButtonVariant::Normal,
    ));
    footer.spawn(button(
        localization
            .text(language, "newWorld.createWorld")
            .to_owned(),
        NewWorldFooterAction::CreateWorld,
        px(MENU_BUTTON_WIDTH),
        MENU_BUTTON_HEIGHT,
        ButtonVariant::Primary,
    ));
}

pub(super) fn handle_new_world_settings_control_focus(
    interactions: NewWorldGeneralControlInteractions,
    mut seed_input: ResMut<SeedInputState>,
    mut biome_size_input: ResMut<BiomeSizeMultiplierInputState>,
    mut spawn_biome_dropdown: ResMut<SpawnBiomeDropdownState>,
) {
    if !interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    seed_input.reset();
    biome_size_input.reset();
    spawn_biome_dropdown.close();
}

pub(super) fn handle_world_generation_mode_buttons(
    interactions: Query<(&Interaction, &WorldGenerationModeButton), Changed<Interaction>>,
    mut config: ResMut<NewWorldConfig>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed
            && config.world_generation().mode() != button.0
        {
            config.set_world_generation_mode(button.0);
            let enabled_by_default = button.0 == WorldGenerationMode::Normal;
            config.set_worldgen_hydrology(
                enabled_by_default,
                enabled_by_default,
                enabled_by_default,
                enabled_by_default,
            );
        }
    }
}

pub(super) fn handle_world_generation_feature_toggles(
    interactions: Query<(&Interaction, &WorldGenerationFeatureToggle), Changed<Interaction>>,
    mut config: ResMut<NewWorldConfig>,
) {
    if config.world_generation().mode() == WorldGenerationMode::Void {
        return;
    }

    for (interaction, feature) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let settings = config.world_generation();
        match feature {
            WorldGenerationFeatureToggle::Caves => config.set_spawn_caves(!settings.spawn_caves()),
            WorldGenerationFeatureToggle::Rivers => {
                config.set_spawn_rivers(!settings.spawn_rivers())
            }
            WorldGenerationFeatureToggle::Lakes => config.set_spawn_lakes(!settings.spawn_lakes()),
            WorldGenerationFeatureToggle::Oceans => {
                config.set_spawn_oceans(!settings.spawn_oceans())
            }
        }
        break;
    }
}

pub(super) fn handle_spawn_structures_toggle(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SpawnStructuresToggle>)>,
    mut config: ResMut<NewWorldConfig>,
) {
    if interactions.iter().any(|interaction| *interaction == Interaction::Pressed) {
        let next = !config.world_generation().spawn_structures();
        config.set_spawn_structures(next);
    }
}

pub(super) fn handle_single_biome_toggle(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SingleBiomeToggle>)>,
    mut config: ResMut<NewWorldConfig>,
) {
    if interactions.iter().any(|interaction| *interaction == Interaction::Pressed) {
        let next = !config.world_generation().single_biome();
        config.set_single_biome(next);
    }
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
    footer: NewWorldFooterInput,
    mut draft: NewWorldDraft,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut transition: ResMut<ScreenTransition>,
) {
    if !footer.is_new_world() || transition.is_active() {
        return;
    }

    let action = footer.pressed_action();
    let active_name = draft
        .name_input
        .single()
        .ok()
        .is_some_and(|(entity, _)| draft.focus.get() == Some(entity));
    if footer.escape_pressed() && active_name {
        draft.focus.clear();
        return;
    }
    if matches!(action, Some(NewWorldFooterAction::Return))
        || (footer.escape_pressed() && !draft.input_editing())
    {
        transition.request(ScreenTransitionTarget::game(GameState::StartingScreen));
        return;
    }

    if !matches!(action, Some(NewWorldFooterAction::CreateWorld)) {
        return;
    }

    if draft.config.world_generation().single_biome() && draft.config.spawn_biome().is_none() {
        draft.name_feedback.set(
            localization
                .text(language.get(), "newWorld.singleBiome.required")
                .to_owned(),
        );
        draft.spawn_biome_dropdown.open();
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
    let (name, session_lock) = match create_new_world(
        &requested,
        seed,
        &dimension.id,
        draft.config.biome_size_multiplier(),
        rules.ticks_per_second(),
        rules.spawn_creatures(),
        draft.config.world_generation(),
    ) {
        Ok(created) => created,
        Err(error) => {
            draft.name_feedback.set(error.to_string());
            return;
        }
    };
    draft.config.set_name(name);
    draft.name_feedback.set(String::new());

    commands.insert_resource(session_lock);
    commands.insert_resource(dimension);
    commands.insert_resource(CurrentBiome::default());
    commands.insert_resource(WorldSeed(seed));
    commands.insert_resource(rules);
    commands.insert_resource(WorldLoadMode::New);
    transition.request(ScreenTransitionTarget::game(GameState::Loading));
}

pub(super) fn sync_world_generation_mode_buttons(
    config: Res<NewWorldConfig>,
    mut buttons: Query<
        (
            &WorldGenerationModeButton,
            Ref<Interaction>,
            &mut ButtonVariant,
        ),
    >,
) {
    let selected = config.world_generation().mode();
    for (button, interaction, mut variant) in &mut buttons {
        if !config.is_changed() && !interaction.is_changed() {
            continue;
        }
        *variant = ButtonVariant::from_active(button.0 == selected);
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn sync_world_generation_toggles(
    config: Res<NewWorldConfig>,
    mut structure_toggles: Query<
        (Ref<Interaction>, &mut BackgroundColor, &mut BorderColor),
        (With<SpawnStructuresToggle>, Without<SingleBiomeToggle>),
    >,
    mut structure_thumbs: Query<
        &mut Node,
        (With<SpawnStructuresToggleThumb>, Without<SingleBiomeToggleThumb>),
    >,
    mut single_toggles: Query<
        (Ref<Interaction>, &mut BackgroundColor, &mut BorderColor),
        (With<SingleBiomeToggle>, Without<SpawnStructuresToggle>),
    >,
    mut single_thumbs: Query<
        &mut Node,
        (With<SingleBiomeToggleThumb>, Without<SpawnStructuresToggleThumb>),
    >,
) {
    let structures = config.world_generation().spawn_structures();
    let single = config.world_generation().single_biome();

    for (interaction, background, border) in &mut structure_toggles {
        if config.is_changed() || interaction.is_changed() {
            selectable::apply_colors(
                toggle::colors(structures, *interaction),
                background,
                border,
            );
        }
    }
    for (interaction, background, border) in &mut single_toggles {
        if config.is_changed() || interaction.is_changed() {
            selectable::apply_colors(
                toggle::colors(single, *interaction),
                background,
                border,
            );
        }
    }

    if config.is_changed() {
        let structures_left = px(toggle::thumb_left(structures));
        for mut thumb in &mut structure_thumbs {
            thumb.left = structures_left;
        }
        let single_left = px(toggle::thumb_left(single));
        for mut thumb in &mut single_thumbs {
            thumb.left = single_left;
        }
    }
}

pub(super) fn sync_world_generation_feature_toggles(
    config: Res<NewWorldConfig>,
    mut toggles: Query<(
        &WorldGenerationFeatureToggle,
        Ref<Interaction>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut thumbs: Query<(&WorldGenerationFeatureToggleThumb, &mut Node)>,
) {
    let settings = config.world_generation();
    let disabled = settings.mode() == WorldGenerationMode::Void;
    let enabled = |feature: WorldGenerationFeatureToggle| match feature {
        WorldGenerationFeatureToggle::Caves => settings.spawn_caves(),
        WorldGenerationFeatureToggle::Rivers => settings.spawn_rivers(),
        WorldGenerationFeatureToggle::Lakes => settings.spawn_lakes(),
        WorldGenerationFeatureToggle::Oceans => settings.spawn_oceans(),
    };

    for (feature, interaction, mut background, mut border) in &mut toggles {
        if !config.is_changed() && !interaction.is_changed() {
            continue;
        }
        let (next_background, next_border) = if disabled {
            (theme::SURFACE_INSET, theme::BORDER)
        } else {
            toggle::colors(enabled(*feature), *interaction)
        };
        background.0 = next_background;
        *border = BorderColor::all(next_border);
    }

    if config.is_changed() {
        for (thumb, mut node) in &mut thumbs {
            node.left = px(toggle::thumb_left(enabled(thumb.0)));
        }
    }
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
