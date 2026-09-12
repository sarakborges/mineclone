use bevy::{prelude::*, ui_widgets::ScrollArea};

use crate::{
    app::settings_state::SettingsState,
    localization::{ActiveLanguage, UiLocalization},
    player::{camera::GameplayCamera, game_mode::GameMode},
    ui::{
        button::menu_button,
        cosmic_background::{self, STAR_FIELD},
        surface, theme, typography,
    },
    world::{
        InMemoryWorldSave, game_rules::GameRules,
        render_distance::RenderDistanceSettings,
    },
};

use super::{
    game_rules_section::game_rules_section,
    languages_section::languages_section,
    navigation::{
        SettingsBackButton, SettingsSection, SettingsSectionPanel, SettingsSectionSelection,
        section_button,
    },
    render_distance_section::graphics_section,
    scroll_area::vertical_scrollbar,
    world_settings_section::world_settings_section,
};

const CONTENT_WIDTH: f32 = 1120.0;
const SIDEBAR_WIDTH: f32 = 280.0;
const HEADER_HEIGHT: f32 = 116.0;
const FOOTER_HEIGHT: f32 = 104.0;
const COLUMN_GAP: f32 = 22.0;

pub fn spawn_settings_screen(
    mut commands: Commands,
    render_distance: Res<RenderDistanceSettings>,
    game_rules: Res<GameRules>,
    save: Res<InMemoryWorldSave>,
    localization: Res<UiLocalization>,
    active_language: Res<ActiveLanguage>,
    player: Query<&GameMode, With<GameplayCamera>>,
    mut selection: ResMut<SettingsSectionSelection>,
) {
    let has_world = save.has_world();
    selection.selected = if has_world {
        SettingsSection::WorldSettings
    } else {
        SettingsSection::Graphics
    };

    let language = active_language.get();
    let game_mode = player
        .single()
        .map_or_else(|_| GameMode::default(), |game_mode| *game_mode);

    commands
        .spawn((
            DespawnOnExit(SettingsState::Open),
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
                    localization.text(language, "settings.title").to_owned(),
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
                                                row_gap: px(8),
                                                overflow: Overflow::scroll_y(),
                                                ..default()
                                            },
                                            ScrollPosition(Vec2::ZERO),
                                            ScrollArea,
                                        ))
                                        .with_children(|list| {
                                            if has_world {
                                                spawn_section_button(
                                                    list,
                                                    SettingsSection::WorldSettings,
                                                    selection.selected,
                                                    &localization,
                                                    language,
                                                );
                                                spawn_section_button(
                                                    list,
                                                    SettingsSection::GameRules,
                                                    selection.selected,
                                                    &localization,
                                                    language,
                                                );
                                            }
                                            spawn_section_button(
                                                list,
                                                SettingsSection::Graphics,
                                                selection.selected,
                                                &localization,
                                                language,
                                            );
                                            spawn_section_button(
                                                list,
                                                SettingsSection::Languages,
                                                selection.selected,
                                                &localization,
                                                language,
                                            );
                                        })
                                        .id();

                                    frame.spawn(vertical_scrollbar(scroll_area_id));
                                });
                        });

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
                                        .with_children(|panels| {
                                            if has_world {
                                                panels
                                                    .spawn((
                                                        SettingsSectionPanel(
                                                            SettingsSection::WorldSettings,
                                                        ),
                                                        section_panel_node(
                                                            selection.selected
                                                                == SettingsSection::WorldSettings,
                                                        ),
                                                    ))
                                                    .with_child(world_settings_section(
                                                        game_mode,
                                                        &localization,
                                                        language,
                                                    ));

                                                panels
                                                    .spawn((
                                                        SettingsSectionPanel(
                                                            SettingsSection::GameRules,
                                                        ),
                                                        section_panel_node(
                                                            selection.selected
                                                                == SettingsSection::GameRules,
                                                        ),
                                                    ))
                                                    .with_child(game_rules_section(
                                                        game_rules.ticks_per_second(),
                                                        &localization,
                                                        language,
                                                    ));
                                            }

                                            panels
                                                .spawn((
                                                    SettingsSectionPanel(SettingsSection::Graphics),
                                                    section_panel_node(
                                                        selection.selected
                                                            == SettingsSection::Graphics,
                                                    ),
                                                ))
                                                .with_child(graphics_section(
                                                    render_distance.chunks(),
                                                    &localization,
                                                    language,
                                                ));

                                            panels
                                                .spawn((
                                                    SettingsSectionPanel(SettingsSection::Languages),
                                                    section_panel_node(
                                                        selection.selected
                                                            == SettingsSection::Languages,
                                                    ),
                                                ))
                                                .with_child(languages_section(
                                                    &localization,
                                                    language,
                                                ));
                                        })
                                        .id();

                                    frame.spawn(vertical_scrollbar(scroll_area_id));
                                });
                        });
                });
            });

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                bottom: px(0),
                height: px(FOOTER_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|footer| {
                footer.spawn(menu_button(
                    localization.text(language, "settings.return").to_owned(),
                    SettingsBackButton,
                ));
            });
        });
}

fn spawn_section_button(
    parent: &mut ChildSpawnerCommands,
    section: SettingsSection,
    selected: SettingsSection,
    localization: &UiLocalization,
    language: crate::localization::Language,
) {
    parent.spawn(section_button(
        section,
        selected == section,
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
