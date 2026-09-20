use bevy::{prelude::*, ui_widgets::ScrollArea};

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, UiLocalization},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        cosmic_background::{self, STAR_FIELD},
        screen, scrollbar, surface, theme, typography,
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
    language: crate::localization::Language,
) {
    let id = world.id.clone();
    parent
        .spawn((
            WorldListEntry(id.clone()),
            Node {
                width: percent(100),
                min_height: px(68),
                padding: UiRect::all(px(10)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                border: UiRect::all(px(2)),
                ..default()
            },
            BackgroundColor(theme::SURFACE_INSET),
            BorderColor::all(theme::BORDER),
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                children![
                    typography::setting_title(world.id.clone()),
                    typography::caption(format_save_time(world.last_saved_unix_ms)),
                ],
            ));
            row.spawn(button(
                localization.text(language, "worldSelection.load").to_owned(),
                WorldSelectionAction::Load(id.clone()),
                px(160),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Primary,
            ));
            row.spawn(button(
                localization.text(language, "worldSelection.delete").to_owned(),
                WorldSelectionAction::Delete(id),
                px(160),
                COMPACT_CONTROL_HEIGHT,
                ButtonVariant::Danger,
            ));
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
        BackgroundColor(theme::SCREEN_BACKGROUND),
        theme::cosmic_background_gradient(),
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
                        .spawn(surface::settings_content())
                        .with_children(|panel| {
                            panel
                                .spawn(Node {
                                    width: percent(100),
                                    height: percent(100),
                                    min_height: px(0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Stretch,
                                    row_gap: px(12),
                                    ..default()
                                })
                                .with_children(|column| {
                                    column.spawn((
                                        WorldListStatus,
                                        typography::caption(if state.scan.is_some() {
                                            localization
                                                .text(
                                                    language.get(),
                                                    "worldSelection.verifying",
                                                )
                                                .to_owned()
                                        } else {
                                            localization
                                                .text(
                                                    language.get(),
                                                    "worldSelection.noRestorable",
                                                )
                                                .to_owned()
                                        }),
                                    ));

                                    column
                                        .spawn(Node {
                                            display: Display::Grid,
                                            flex_grow: 1.0,
                                            min_height: px(0),
                                            width: percent(100),
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
                                                        row_gap: px(10),
                                                        overflow: Overflow::scroll_y(),
                                                        ..default()
                                                    },
                                                ))
                                                .id();
                                            frame.spawn(scrollbar::vertical_scrollbar(list_id));
                                        });

                                    column.spawn((
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
