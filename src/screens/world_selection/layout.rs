use bevy::{prelude::*, ui_widgets::ScrollArea};

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, dimension::DimensionRegistry},
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        cosmic_background::{self, STAR_FIELD},
        screen, scrollbar, surface, typography,
    },
    world::save_catalog::WorldSummary,
};

use super::{WorldSelectionAction, WorldSelectionState};

#[derive(Component)]
pub(super) struct WorldListEntry(pub(super) String);

#[derive(Component)]
pub(super) struct SelectionError;

#[derive(Component)]
pub(super) struct WorldListStatus;

#[derive(Component)]
pub(super) struct WorldListContainer;

pub(super) fn spawn_world_entry(
    parent: &mut ChildSpawnerCommands,
    world: &WorldSummary,
    localization: &UiLocalization,
    language: Language,
    dimensions: &DimensionRegistry,
    biomes: &BiomeRegistry,
) {
    let id = world.id.clone();
    let dimension = dimensions
        .get(&world.dimension_id)
        .map_or(world.dimension_id.as_str(), |definition| {
            definition.name.text(language)
        });
    let biome = world
        .biome_id
        .as_deref()
        .and_then(|biome_id| biomes.get(biome_id))
        .map_or(
            localization.text(language, "worldSelection.unknown"),
            |definition| definition.name.text(language),
        );
    let position = format_coordinates(world.player_position);
    let days_passed = world.day.saturating_sub(1);

    let mut card = parent.spawn(surface::settings_content());
    card.insert(WorldListEntry(id.clone()));
    card.with_children(|card| {
        card.spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(20),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                flex_grow: 1.0,
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            })
            .with_children(|info| {
                info.spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|header| {
                    header.spawn(typography::heading(world.id.clone()));
                    header.spawn(typography::caption(format_save_time(
                        world.last_saved_unix_ms,
                    )));
                });

                info.spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(24),
                    row_gap: px(10),
                    ..default()
                })
                .with_children(|metadata| {
                    spawn_metadata(
                        metadata,
                        localization.text(language, "worldSelection.seed"),
                        world.seed.to_string(),
                    );
                    spawn_metadata(
                        metadata,
                        localization.text(language, "worldSelection.daysPassed"),
                        days_passed.to_string(),
                    );
                    spawn_metadata(
                        metadata,
                        localization.text(language, "worldSelection.dimension"),
                        dimension.to_owned(),
                    );
                    spawn_metadata(
                        metadata,
                        localization.text(language, "worldSelection.coordinates"),
                        position,
                    );
                    spawn_metadata(
                        metadata,
                        localization.text(language, "worldSelection.biome"),
                        biome.to_owned(),
                    );
                });
            });

            row.spawn(Node {
                width: px(170),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            })
            .with_children(|actions| {
                actions.spawn(button(
                    localization.text(language, "worldSelection.load").to_owned(),
                    WorldSelectionAction::Load(id.clone()),
                    percent(100),
                    COMPACT_CONTROL_HEIGHT,
                    ButtonVariant::Primary,
                ));
                actions.spawn(button(
                    localization.text(language, "worldSelection.delete").to_owned(),
                    WorldSelectionAction::Delete(id),
                    percent(100),
                    COMPACT_CONTROL_HEIGHT,
                    ButtonVariant::Danger,
                ));
            });
        });
    });
}

fn spawn_metadata(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    value: impl Into<String>,
) {
    parent
        .spawn(Node {
            min_width: px(150),
            max_width: px(260),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            ..default()
        })
        .with_children(|item| {
            item.spawn(typography::caption(label.to_owned()));
            item.spawn(typography::muted(value.into()));
        });
}

pub(super) fn spawn_world_selection(
    mut commands: Commands,
    state: Res<WorldSelectionState>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((
        Camera2d,
        BoxShadowSamples(8),
        DespawnOnExit(GameState::WorldSelection),
    ));
    let mut root = commands.spawn((
        DespawnOnExit(GameState::WorldSelection),
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
        crate::ui::theme::cosmic_background_gradient(),
    ));

    root.with_children(|root| {
        for &spec in STAR_FIELD {
            root.spawn(cosmic_background::star(spec));
        }

        root.spawn(screen::header()).with_children(|header| {
            header.spawn(typography::title(
                localization
                    .text(language.get(), "starting.loadWorlds")
                    .to_owned(),
            ));
        });

        root.spawn(screen::body()).with_children(|body| {
            body.spawn(screen::content_column(12.0))
                .with_children(|content| {
                    content
                        .spawn(Node {
                            position_type: PositionType::Relative,
                            width: percent(100),
                            flex_grow: 1.0,
                            min_height: px(0),
                            display: Display::Grid,
                            grid_template_columns: vec![
                                RepeatedGridTrack::flex(1, 1.0),
                                RepeatedGridTrack::auto(1),
                            ],
                            ..default()
                        })
                        .with_children(|frame| {
                            let list_id = frame
                                .spawn((
                                    WorldListContainer,
                                    ScrollPosition(Vec2::ZERO),
                                    ScrollArea,
                                    Node {
                                        width: percent(100),
                                        height: percent(100),
                                        min_height: px(0),
                                        padding: UiRect::right(px(12)),
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Stretch,
                                        row_gap: px(12),
                                        overflow: Overflow::scroll_y(),
                                        ..default()
                                    },
                                ))
                                .id();
                            frame.spawn(scrollbar::vertical_scrollbar(list_id));

                            let status = if state.scan.is_some() {
                                localization
                                    .text(language.get(), "worldSelection.verifying")
                                    .to_owned()
                            } else {
                                localization
                                    .text(language.get(), "worldSelection.noRestorable")
                                    .to_owned()
                            };
                            frame.spawn((
                                WorldListStatus,
                                typography::heading(status),
                                TextLayout::new_with_justify(Justify::Center),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(24),
                                    right: px(48),
                                    top: px(0),
                                    bottom: px(0),
                                    display: if state.scan.is_some() || state.worlds.is_empty() {
                                        Display::Flex
                                    } else {
                                        Display::None
                                    },
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                        });

                    content.spawn((
                        SelectionError,
                        typography::caption(state.error.clone()),
                        Node {
                            display: if state.error.is_empty() {
                                Display::None
                            } else {
                                Display::Flex
                            },
                            ..default()
                        },
                    ));
                });
        });

        root.spawn(screen::footer()).with_children(|footer| {
            footer.spawn(button(
                localization.text(language.get(), "newWorld.return").to_owned(),
                WorldSelectionAction::Back,
                px(360),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Normal,
            ));
            footer.spawn(button(
                localization
                    .text(language.get(), "worldSelection.openSavesFolder")
                    .to_owned(),
                WorldSelectionAction::OpenSavesFolder,
                px(360),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Normal,
            ));
        });
    });
}

fn format_coordinates(position: Option<[f32; 3]>) -> String {
    let Some([x, y, z]) = position else {
        return "—".to_owned();
    };
    format!("X: {:.0} · Y: {:.0} · Z: {:.0}", x.floor(), y.floor(), z.floor())
}

// Portable UTC rendering without relying on local timezone configuration.
fn format_save_time(unix_ms: u64) -> String {
    let seconds = unix_ms / 1_000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let seconds_in_day = seconds % 86_400;
    let z = days.saturating_add(719_468);
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    let hour = seconds_in_day / 3_600;
    let minute = seconds_in_day / 60 % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}
