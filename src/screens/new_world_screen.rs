use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, Language, UiLocalization},
    player::game_mode::GameMode,
    ui::{
        button::menu_button,
        cosmic_background::{self, STAR_FIELD},
        surface, theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
    world::{
        NewWorldConfig, WorldLoadMode, WorldSeed, biome::CurrentBiome,
        dimension::CurrentDimension,
    },
};

const CONTENT_WIDTH: f32 = 1120.0;
const SIDEBAR_WIDTH: f32 = 280.0;
const HEADER_HEIGHT: f32 = 116.0;
const FOOTER_HEIGHT: f32 = 104.0;
const COLUMN_GAP: f32 = 22.0;
const CONTROL_HEIGHT: f32 = 44.0;
const STEP_BUTTON_SIZE: f32 = 44.0;
const TICKS_INPUT_WIDTH: f32 = 124.0;

pub(crate) struct NewWorldScreenPlugin;

impl Plugin for NewWorldScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NewWorldConfig>()
            .init_resource::<NewWorldSectionSelection>()
            .init_resource::<SeedInputState>()
            .init_resource::<TicksInputState>()
            .add_systems(
                OnEnter(GameState::NewWorld),
                (reset_new_world_screen, spawn_new_world_screen).chain(),
            )
            .add_systems(
                Update,
                (
                    handle_section_buttons,
                    handle_seed_focus,
                    handle_seed_keyboard,
                    handle_game_mode_buttons,
                    handle_ticks_step_buttons,
                    handle_ticks_focus,
                    handle_ticks_keyboard,
                    handle_footer_buttons,
                    sync_section_ui,
                    sync_seed_text,
                    sync_game_mode_buttons,
                    sync_ticks_text,
                )
                    .chain()
                    .run_if(in_state(GameState::NewWorld)),
            );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NewWorldSection {
    General,
    GameRules,
}

impl NewWorldSection {
    const fn localization_key(self) -> &'static str {
        match self {
            Self::General => "newWorld.section.general",
            Self::GameRules => "settings.section.gameRules",
        }
    }
}

#[derive(Resource, Clone, Copy)]
struct NewWorldSectionSelection {
    selected: NewWorldSection,
}

impl Default for NewWorldSectionSelection {
    fn default() -> Self {
        Self {
            selected: NewWorldSection::General,
        }
    }
}

#[derive(Component, Clone, Copy)]
struct NewWorldSectionButton(NewWorldSection);

#[derive(Component, Clone, Copy)]
struct NewWorldSectionButtonLabel(NewWorldSection);

#[derive(Component, Clone, Copy)]
struct NewWorldSectionPanel(NewWorldSection);

#[derive(Component)]
struct SeedInput;

#[derive(Component)]
struct SeedValueText;

#[derive(Resource, Default)]
struct SeedInputState {
    editing: bool,
    replace_on_next_digit: bool,
    buffer: String,
}

#[derive(Component, Clone, Copy)]
struct GameModeButton(GameMode);

#[derive(Component, Clone, Copy)]
struct GameModeButtonLabel(GameMode);

#[derive(Component, Clone, Copy)]
enum TicksStep {
    Decrement,
    Increment,
}

#[derive(Component)]
struct TicksInput;

#[derive(Component)]
struct TicksValueText;

#[derive(Resource, Default)]
struct TicksInputState {
    editing: bool,
    replace_on_next_digit: bool,
    buffer: String,
}

#[derive(Component, Clone, Copy)]
enum NewWorldFooterAction {
    Return,
    CreateWorld,
}

fn reset_new_world_screen(
    mut config: ResMut<NewWorldConfig>,
    mut selection: ResMut<NewWorldSectionSelection>,
    mut seed_input: ResMut<SeedInputState>,
    mut ticks_input: ResMut<TicksInputState>,
) {
    config.reset();
    selection.selected = NewWorldSection::General;
    *seed_input = SeedInputState::default();
    *ticks_input = TicksInputState::default();
}

fn spawn_new_world_screen(
    mut commands: Commands,
    config: Res<NewWorldConfig>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    selection: Res<NewWorldSectionSelection>,
) {
    commands.spawn((
        Camera2d,
        BoxShadowSamples(8),
        DespawnOnExit(GameState::NewWorld),
    ));

    let language = active_language.get();

    commands
        .spawn((
            DespawnOnExit(GameState::NewWorld),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
        ))
        .with_children(|root| {
            for &spec in STAR_FIELD {
                root.spawn(cosmic_background::star(spec));
            }

            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(HEADER_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn(typography::title(
                    localization.text(language, "newWorld.title").to_owned(),
                ));
            });

            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(HEADER_HEIGHT),
                bottom: px(FOOTER_HEIGHT),
                left: px(0),
                right: px(0),
                padding: UiRect::axes(px(32), px(18)),
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Center,
                min_height: px(0),
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    width: px(CONTENT_WIDTH),
                    max_width: percent(100),
                    height: percent(100),
                    min_height: px(0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Stretch,
                    column_gap: px(COLUMN_GAP),
                    ..default()
                })
                .with_children(|columns| {
                    columns
                        .spawn(surface::settings_sidebar(SIDEBAR_WIDTH))
                        .with_children(|sidebar| {
                            sidebar.spawn(section_button(
                                NewWorldSection::General,
                                selection.selected == NewWorldSection::General,
                                &localization,
                                language,
                            ));
                            sidebar.spawn(section_button(
                                NewWorldSection::GameRules,
                                selection.selected == NewWorldSection::GameRules,
                                &localization,
                                language,
                            ));
                        });

                    columns
                        .spawn(surface::settings_content())
                        .with_children(|content| {
                            content
                                .spawn((
                                    NewWorldSectionPanel(NewWorldSection::General),
                                    panel_node(selection.selected == NewWorldSection::General),
                                ))
                                .with_child(general_section(
                                    &config,
                                    &localization,
                                    language,
                                ));

                            content
                                .spawn((
                                    NewWorldSectionPanel(NewWorldSection::GameRules),
                                    panel_node(selection.selected == NewWorldSection::GameRules),
                                ))
                                .with_child(game_rules_section(
                                    config.game_rules().ticks_per_second(),
                                    &localization,
                                    language,
                                ));
                        });
                });
            });

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                bottom: px(0),
                height: px(FOOTER_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(18),
                ..default()
            })
            .with_children(|footer| {
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
            });
        });
}

fn section_button(
    section: NewWorldSection,
    active: bool,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        Button,
        NewWorldSectionButton(section),
        Node {
            width: percent(100),
            min_height: px(48),
            padding: UiRect::axes(px(16), px(10)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(section_button_background(active, Interaction::None)),
        children![(
            typography::button_label(
                localization
                    .text(language, section.localization_key())
                    .to_owned(),
            ),
            NewWorldSectionButtonLabel(section),
        )],
    )
}

fn general_section(
    config: &NewWorldConfig,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        surface::settings_section(),
        children![
            typography::heading(
                localization
                    .text(language, "newWorld.section.general")
                    .to_owned(),
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                },
                children![
                    typography::muted(localization.text(language, "newWorld.seed").to_owned()),
                    seed_input(config.seed().0),
                ],
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                },
                children![
                    typography::muted(
                        localization.text(language, "settings.gameMode").to_owned(),
                    ),
                    (
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(12),
                            ..default()
                        },
                        children![
                            game_mode_button(
                                localization
                                    .text(language, "settings.gameMode.survival")
                                    .to_owned(),
                                GameMode::Survival,
                                config.game_mode(),
                            ),
                            game_mode_button(
                                localization
                                    .text(language, "settings.gameMode.creative")
                                    .to_owned(),
                                GameMode::Creative,
                                config.game_mode(),
                            ),
                        ],
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
            width: percent(100),
            height: px(CONTROL_HEIGHT),
            border: UiRect::all(px(1)),
            padding: UiRect::axes(px(14), px(0)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.045, 0.035, 0.09, 0.88)),
        BorderColor::all(Color::srgba(0.43, 0.36, 0.68, 0.72)),
        children![(
            typography::button_label(seed.to_string()),
            SeedValueText,
        )],
    )
}

fn game_mode_button(
    label: impl Into<String>,
    mode: GameMode,
    current: GameMode,
) -> impl Bundle {
    let active = mode == current;

    (
        Button,
        GameModeButton(mode),
        Node {
            flex_grow: 1.0,
            height: px(CONTROL_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(game_mode_button_background(active, Interaction::None)),
        children![(typography::button_label(label), GameModeButtonLabel(mode))],
    )
}

fn game_rules_section(
    ticks_per_second: u32,
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    (
        surface::settings_section(),
        children![
            typography::heading(
                localization
                    .text(language, "settings.section.gameRules")
                    .to_owned(),
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    ..default()
                },
                children![
                    typography::muted(
                        localization
                            .text(language, "settings.ticksBySecond")
                            .to_owned(),
                    ),
                    (
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(10),
                            ..default()
                        },
                        children![
                            ticks_step_button("−", TicksStep::Decrement),
                            ticks_input(ticks_per_second),
                            ticks_step_button("+", TicksStep::Increment),
                        ],
                    ),
                    typography::caption(
                        localization
                            .text(language, "settings.ticksBySecond.description")
                            .to_owned(),
                    ),
                ],
            ),
        ],
    )
}

fn ticks_step_button(label: &'static str, step: TicksStep) -> impl Bundle {
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
        TicksInput,
        Node {
            width: px(TICKS_INPUT_WIDTH),
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
            TicksValueText,
        )],
    )
}

fn panel_node(visible: bool) -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        display: if visible {
            Display::Flex
        } else {
            Display::None
        },
        ..default()
    }
}

fn handle_section_buttons(
    interactions: Query<(&Interaction, &NewWorldSectionButton), Changed<Interaction>>,
    mut selection: ResMut<NewWorldSectionSelection>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            selection.selected = button.0;
        }
    }
}

fn handle_seed_focus(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SeedInput>)>,
    config: Res<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        input.editing = true;
        input.replace_on_next_digit = true;
        input.buffer = config.seed().0.to_string();
    }
}

fn handle_seed_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<NewWorldConfig>,
    mut input: ResMut<SeedInputState>,
) {
    if !input.editing {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Escape) {
        input.editing = false;
        input.replace_on_next_digit = false;
        input.buffer.clear();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
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

        if next.parse::<u64>().is_ok() {
            input.buffer = next;
            input.replace_on_next_digit = false;
            apply_seed_buffer(&input.buffer, &mut config);
        }
        break;
    }
}

fn handle_game_mode_buttons(
    interactions: Query<(&Interaction, &GameModeButton), Changed<Interaction>>,
    mut config: ResMut<NewWorldConfig>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            config.set_game_mode(button.0);
        }
    }
}

fn handle_ticks_step_buttons(
    interactions: Query<(&Interaction, &TicksStep), Changed<Interaction>>,
    mut config: ResMut<NewWorldConfig>,
    mut input: ResMut<TicksInputState>,
) {
    for (interaction, step) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let current = config.game_rules().ticks_per_second();
        let next = match step {
            TicksStep::Decrement => current.saturating_sub(1).max(1),
            TicksStep::Increment => current.saturating_add(1),
        };
        config.set_ticks_per_second(next);
        input.editing = false;
        input.replace_on_next_digit = false;
        input.buffer.clear();
    }
}

fn handle_ticks_focus(
    interactions: Query<&Interaction, (Changed<Interaction>, With<TicksInput>)>,
    config: Res<NewWorldConfig>,
    mut input: ResMut<TicksInputState>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        input.editing = true;
        input.replace_on_next_digit = true;
        input.buffer = config.game_rules().ticks_per_second().to_string();
    }
}

fn handle_ticks_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<NewWorldConfig>,
    mut input: ResMut<TicksInputState>,
) {
    if !input.editing {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Escape) {
        input.editing = false;
        input.replace_on_next_digit = false;
        input.buffer.clear();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        if input.replace_on_next_digit {
            input.buffer.clear();
            input.replace_on_next_digit = false;
        } else {
            input.buffer.pop();
        }
        apply_ticks_buffer(&input.buffer, &mut config);
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

        if let Ok(value) = next.parse::<u32>()
            && value > 0
        {
            input.buffer = next;
            input.replace_on_next_digit = false;
            config.set_ticks_per_second(value);
        }
        break;
    }
}

fn handle_footer_buttons(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&Interaction, &NewWorldFooterAction), Changed<Interaction>>,
    mut config: ResMut<NewWorldConfig>,
    seed_input: Res<SeedInputState>,
    ticks_input: Res<TicksInputState>,
    mut transition: ResMut<ScreenTransition>,
) {
    let action = interactions.iter().find_map(|(interaction, action)| {
        (*interaction == Interaction::Pressed).then_some(*action)
    });

    let escape_pressed = keys.just_pressed(KeyCode::Escape)
        && !seed_input.editing
        && !ticks_input.editing;

    if escape_pressed || matches!(action, Some(NewWorldFooterAction::Return)) {
        transition.request(ScreenTransitionTarget::game(GameState::StartingScreen));
        return;
    }

    if !matches!(action, Some(NewWorldFooterAction::CreateWorld)) {
        return;
    }

    if let Ok(seed) = seed_input.buffer.parse::<u64>() {
        config.set_seed(seed);
    }

    commands.insert_resource(CurrentDimension::default());
    commands.insert_resource(CurrentBiome::default());
    commands.insert_resource(WorldSeed(config.seed().0));
    commands.insert_resource(WorldLoadMode::New);
    transition.request(ScreenTransitionTarget::game(GameState::Loading));
}

fn sync_section_ui(
    selection: Res<NewWorldSectionSelection>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    mut panels: Query<(&NewWorldSectionPanel, &mut Node)>,
    mut buttons: Query<(&NewWorldSectionButton, &Interaction, &mut BackgroundColor)>,
    mut labels: Query<(&NewWorldSectionButtonLabel, &mut Text, &mut TextColor)>,
) {
    for (panel, mut node) in &mut panels {
        node.display = if panel.0 == selection.selected {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (button, interaction, mut background) in &mut buttons {
        *background = BackgroundColor(section_button_background(
            button.0 == selection.selected,
            *interaction,
        ));
    }

    for (label, mut text, mut color) in &mut labels {
        let next = localization.text(active_language.get(), label.0.localization_key());
        if text.0 != next {
            text.0 = next.to_owned();
        }
        *color = TextColor(if label.0 == selection.selected {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_MUTED
        });
    }
}

fn sync_seed_text(
    config: Res<NewWorldConfig>,
    input: Res<SeedInputState>,
    mut labels: Query<&mut Text, With<SeedValueText>>,
) {
    let value = if input.editing {
        input.buffer.clone()
    } else {
        config.seed().0.to_string()
    };

    for mut label in &mut labels {
        if label.0 != value {
            label.0 = value.clone();
        }
    }
}

fn sync_game_mode_buttons(
    config: Res<NewWorldConfig>,
    mut buttons: Query<(&GameModeButton, &Interaction, &mut BackgroundColor)>,
    mut labels: Query<(&GameModeButtonLabel, &mut TextColor)>,
) {
    let current = config.game_mode();

    for (button, interaction, mut background) in &mut buttons {
        *background = BackgroundColor(game_mode_button_background(
            button.0 == current,
            *interaction,
        ));
    }

    for (label, mut color) in &mut labels {
        *color = TextColor(if label.0 == current {
            theme::TEXT_SUBTLE
        } else {
            theme::TEXT_PRIMARY
        });
    }
}

fn sync_ticks_text(
    config: Res<NewWorldConfig>,
    input: Res<TicksInputState>,
    mut labels: Query<&mut Text, With<TicksValueText>>,
) {
    let value = if input.editing {
        input.buffer.clone()
    } else {
        config.game_rules().ticks_per_second().to_string()
    };

    for mut label in &mut labels {
        if label.0 != value {
            label.0 = value.clone();
        }
    }
}

fn apply_seed_buffer(buffer: &str, config: &mut NewWorldConfig) {
    if let Ok(seed) = buffer.parse::<u64>() {
        config.set_seed(seed);
    }
}

fn apply_ticks_buffer(buffer: &str, config: &mut NewWorldConfig) {
    let Ok(value) = buffer.parse::<u32>() else {
        return;
    };
    if value > 0 {
        config.set_ticks_per_second(value);
    }
}

fn digit_keys() -> [(KeyCode, char); 10] {
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
    ]
}

fn section_button_background(active: bool, interaction: Interaction) -> Color {
    if active {
        return Color::srgba(0.31, 0.20, 0.56, 0.86);
    }

    match interaction {
        Interaction::Pressed => Color::srgba(0.26, 0.18, 0.48, 0.84),
        Interaction::Hovered => Color::srgba(0.20, 0.14, 0.38, 0.78),
        Interaction::None => Color::srgba(0.08, 0.06, 0.16, 0.34),
    }
}

fn game_mode_button_background(active: bool, interaction: Interaction) -> Color {
    if active {
        return Color::srgba(0.08, 0.07, 0.12, 0.62);
    }

    match interaction {
        Interaction::Pressed => Color::srgba(0.34, 0.22, 0.62, 0.92),
        Interaction::Hovered => Color::srgba(0.29, 0.19, 0.54, 0.82),
        Interaction::None => Color::srgba(0.20, 0.14, 0.38, 0.72),
    }
}
