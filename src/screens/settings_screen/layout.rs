use bevy::{prelude::*, ui_widgets::ScrollArea};

use crate::{
    app::{game_state::GameState, settings_state::SettingsState},
    hud::HudSettings,
    localization::{ActiveLanguage, Language, UiLocalization},
    player::{camera::GameplayCamera, game_mode::GameMode},
    ui::{
        button::menu_button,
        cosmic_background::{self, STAR_FIELD},
        scrollbar::vertical_scrollbar,
        surface, theme, typography,
    },
    world::{
        NewWorldConfig, game_rules::GameRules,
        render_distance::RenderDistanceSettings,
    },
};

use super::{
    game_rules_section::game_rules_section,
    languages_section::languages_section,
    miscellaneous_section::miscellaneous_section,
    navigation::{
        SettingsBackButton, SettingsSection, SettingsSectionPanel, SettingsSectionSelection,
        section_button,
    },
    new_world_section::{new_world_general_section, spawn_new_world_footer},
    render_distance_section::graphics_section,
    world_settings_section::world_settings_section,
};

const CONTENT_WIDTH: f32 = 1120.0;
const SIDEBAR_WIDTH: f32 = 280.0;
const HEADER_HEIGHT: f32 = 116.0;
const FOOTER_HEIGHT: f32 = 104.0;
const COLUMN_GAP: f32 = 22.0;
const SIDEBAR_BUTTON_GAP: f32 = 11.0;

const START_SECTIONS: &[SettingsSection] = &[
    SettingsSection::Graphics,
    SettingsSection::Languages,
    SettingsSection::Miscellaneous,
];
const IN_WORLD_SECTIONS: &[SettingsSection] = &[
    SettingsSection::WorldSettings,
    SettingsSection::GameRules,
    SettingsSection::Graphics,
    SettingsSection::Languages,
    SettingsSection::Miscellaneous,
];
const CREATE_WORLD_SECTIONS: &[SettingsSection] = &[
    SettingsSection::General,
    SettingsSection::GameRules,
];

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsScreenContext {
    Start,
    InWorld,
    CreateWorld,
}

impl SettingsScreenContext {
    fn from_game_state(game_state: GameState) -> Self {
        match game_state {
            GameState::NewWorld => Self::CreateWorld,
            GameState::Gameplay => Self::InWorld,
            _ => Self::Start,
        }
    }

    const fn title_key(self) -> &'static str {
        match self {
            Self::CreateWorld => "newWorld.title",
            Self::Start | Self::InWorld => "settings.title",
        }
    }

    const fn initial_section(self) -> SettingsSection {
        match self {
            Self::Start => SettingsSection::Graphics,
            Self::InWorld => SettingsSection::WorldSettings,
            Self::CreateWorld => SettingsSection::General,
        }
    }

    const fn sections(self) -> &'static [SettingsSection] {
        match self {
            Self::Start => START_SECTIONS,
            Self::InWorld => IN_WORLD_SECTIONS,
            Self::CreateWorld => CREATE_WORLD_SECTIONS,
        }
    }
}

pub fn spawn_settings_screen(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    render_distance: Res<RenderDistanceSettings>,
    game_rules: Res<GameRules>,
    new_world: Res<NewWorldConfig>,
    hud_settings: Res<HudSettings>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    player: Query<&GameMode, With<GameplayCamera>>,
    mut selection: ResMut<SettingsSectionSelection>,
) {
    let context = SettingsScreenContext::from_game_state(*game_state.get());
    selection.selected = context.initial_section();

    let language = active_language.get();
    let game_mode = match context {
        SettingsScreenContext::CreateWorld => new_world.game_mode(),
        SettingsScreenContext::InWorld => player
            .single()
            .map_or_else(|_| GameMode::default(), |game_mode| *game_mode),
        SettingsScreenContext::Start => GameMode::default(),
    };
    let ticks_per_second = match context {
        SettingsScreenContext::CreateWorld => new_world.game_rules().ticks_per_second(),
        SettingsScreenContext::Start | SettingsScreenContext::InWorld => {
            game_rules.ticks_per_second()
        }
    };

    if context == SettingsScreenContext::CreateWorld {
        commands.spawn((
            Camera2d,
            BoxShadowSamples(8),
            DespawnOnExit(GameState::NewWorld),
        ));
    }

    let mut root = commands.spawn((
        context,
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
        GlobalZIndex(500),
    ));

    match context {
        SettingsScreenContext::CreateWorld => {
            root.insert(DespawnOnExit(GameState::NewWorld));
        }
        SettingsScreenContext::Start | SettingsScreenContext::InWorld => {
            root.insert(DespawnOnExit(SettingsState::Open));
        }
    }

    root.with_children(|root| {
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
                localization.text(language, context.title_key()).to_owned(),
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
                spawn_sidebar(columns, context, &localization, language);
                spawn_content(
                    columns,
                    context,
                    &render_distance,
                    &game_rules,
                    &new_world,
                    &hud_settings,
                    game_mode,
                    &localization,
                    language,
                    selection.selected,
                    ticks_per_second,
                );
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
        .with_children(|footer| match context {
            SettingsScreenContext::CreateWorld => {
                spawn_new_world_footer(footer, &localization, language);
            }
            SettingsScreenContext::Start | SettingsScreenContext::InWorld => {
                footer.spawn(menu_button(
                    localization.text(language, "settings.return").to_owned(),
                    SettingsBackButton,
                ));
            }
        });
    });
}

fn spawn_sidebar(
    columns: &mut ChildSpawnerCommands,
    context: SettingsScreenContext,
    localization: &UiLocalization,
    language: Language,
) {
    columns
        .spawn(Node {
            width: px(SIDEBAR_WIDTH),
            height: percent(100),
            min_height: px(0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        })
        .with_children(|sidebar| {
            sidebar
                .spawn(Node {
                    display: Display::Grid,
                    width: percent(100),
                    height: percent(100),
                    min_height: px(0),
                    grid_template_columns: vec![
                        RepeatedGridTrack::flex(1, 1.0),
                        RepeatedGridTrack::auto(1),
                    ],
                    ..default()
                })
                .with_children(|frame| {
                    let scroll_area_id = frame
                        .spawn((
                            Node {
                                width: percent(100),
                                height: percent(100),
                                min_height: px(0),
                                padding: UiRect::right(px(12)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                row_gap: px(SIDEBAR_BUTTON_GAP),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ScrollPosition(Vec2::ZERO),
                            ScrollArea,
                        ))
                        .with_children(|list| {
                            for &section in context.sections() {
                                spawn_section_button(list, section, localization, language);
                            }
                        })
                        .id();

                    frame.spawn(vertical_scrollbar(scroll_area_id));
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_content(
    columns: &mut ChildSpawnerCommands,
    context: SettingsScreenContext,
    render_distance: &RenderDistanceSettings,
    game_rules: &GameRules,
    new_world: &NewWorldConfig,
    hud_settings: &HudSettings,
    game_mode: GameMode,
    localization: &UiLocalization,
    language: Language,
    selected: SettingsSection,
    ticks_per_second: u32,
) {
    columns
        .spawn(surface::settings_content())
        .with_children(|content| {
            content
                .spawn(Node {
                    display: Display::Grid,
                    width: percent(100),
                    height: percent(100),
                    min_height: px(0),
                    grid_template_columns: vec![
                        RepeatedGridTrack::flex(1, 1.0),
                        RepeatedGridTrack::auto(1),
                    ],
                    ..default()
                })
                .with_children(|frame| {
                    let scroll_area_id = frame
                        .spawn((
                            Node {
                                width: percent(100),
                                height: percent(100),
                                min_height: px(0),
                                padding: UiRect::right(px(14)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ScrollPosition(Vec2::ZERO),
                            ScrollArea,
                        ))
                        .with_children(|panels| match context {
                            SettingsScreenContext::CreateWorld => {
                                panels
                                    .spawn((
                                        SettingsSectionPanel(SettingsSection::General),
                                        section_panel_node(selected == SettingsSection::General),
                                    ))
                                    .with_child(new_world_general_section(
                                        new_world,
                                        localization,
                                        language,
                                    ));

                                panels
                                    .spawn((
                                        SettingsSectionPanel(SettingsSection::GameRules),
                                        section_panel_node(selected == SettingsSection::GameRules),
                                    ))
                                    .with_child(game_rules_section(
                                        ticks_per_second,
                                        localization,
                                        language,
                                    ));
                            }
                            SettingsScreenContext::InWorld => {
                                panels
                                    .spawn((
                                        SettingsSectionPanel(SettingsSection::WorldSettings),
                                        section_panel_node(
                                            selected == SettingsSection::WorldSettings,
                                        ),
                                    ))
                                    .with_child(world_settings_section(
                                        game_mode,
                                        localization,
                                        language,
                                    ));

                                panels
                                    .spawn((
                                        SettingsSectionPanel(SettingsSection::GameRules),
                                        section_panel_node(selected == SettingsSection::GameRules),
                                    ))
                                    .with_child(game_rules_section(
                                        game_rules.ticks_per_second(),
                                        localization,
                                        language,
                                    ));

                                spawn_global_settings_panels(
                                    panels,
                                    render_distance,
                                    hud_settings,
                                    localization,
                                    language,
                                    selected,
                                );
                            }
                            SettingsScreenContext::Start => {
                                spawn_global_settings_panels(
                                    panels,
                                    render_distance,
                                    hud_settings,
                                    localization,
                                    language,
                                    selected,
                                );
                            }
                        })
                        .id();

                    frame.spawn(vertical_scrollbar(scroll_area_id));
                });
        });
}

fn spawn_global_settings_panels(
    panels: &mut ChildSpawnerCommands,
    render_distance: &RenderDistanceSettings,
    hud_settings: &HudSettings,
    localization: &UiLocalization,
    language: Language,
    selected: SettingsSection,
) {
    panels
        .spawn((
            SettingsSectionPanel(SettingsSection::Graphics),
            section_panel_node(selected == SettingsSection::Graphics),
        ))
        .with_child(graphics_section(
            render_distance.chunks(),
            localization,
            language,
        ));

    panels
        .spawn((
            SettingsSectionPanel(SettingsSection::Languages),
            section_panel_node(selected == SettingsSection::Languages),
        ))
        .with_child(languages_section(localization, language));

    panels
        .spawn((
            SettingsSectionPanel(SettingsSection::Miscellaneous),
            section_panel_node(selected == SettingsSection::Miscellaneous),
        ))
        .with_child(miscellaneous_section(
            hud_settings.display_tooltips(),
            localization,
            language,
        ));
}

fn spawn_section_button(
    parent: &mut ChildSpawnerCommands,
    section: SettingsSection,
    localization: &UiLocalization,
    language: Language,
) {
    parent.spawn(section_button(
        section,
        localization
            .text(language, section.localization_key())
            .to_owned(),
    ));
}

fn section_panel_node(visible: bool) -> Node {
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
