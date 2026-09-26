use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        cosmic_background::{self, STAR_FIELD},
        surface, theme, typography,
    },
    world::{WorldLoadingPhase, WorldLoadingPhaseStatus, WorldLoadingState},
};

const LOADING_PANEL_WIDTH: f32 = 560.0;
const LOADING_ROW_HEIGHT: f32 = 48.0;
const LOADING_ROW_GAP: f32 = 4.0;
const LOADING_PHASE_INDEX_WIDTH: f32 = 34.0;

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), setup_loading_screen)
            .add_systems(
                Update,
                update_loading_progress.run_if(in_state(GameState::Loading)),
            );
    }
}

#[derive(Component)]
struct LoadingSummaryText;

#[derive(Component)]
struct LoadingPhaseRow(WorldLoadingPhase);

#[derive(Component)]
struct LoadingPhaseLabel(WorldLoadingPhase);

#[derive(Component)]
struct LoadingPhaseStatusText(WorldLoadingPhase);

#[derive(SystemParam)]
struct LoadingProgressUi<'w, 's> {
    summaries: Query<
        'w,
        's,
        &'static mut Text,
        (With<LoadingSummaryText>, Without<LoadingPhaseStatusText>),
    >,
    rows: Query<'w, 's, (&'static LoadingPhaseRow, &'static mut BackgroundColor)>,
    labels: Query<
        'w,
        's,
        (&'static LoadingPhaseLabel, &'static mut TextColor),
        Without<LoadingPhaseStatusText>,
    >,
    statuses: Query<
        'w,
        's,
        (
            &'static LoadingPhaseStatusText,
            &'static mut Text,
            &'static mut TextColor,
        ),
    >,
}

fn setup_loading_screen(
    mut commands: Commands,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((
        Camera2d,
        BoxShadowSamples(8),
        DespawnOnExit(GameState::Loading),
    ));
    let language = language.get();

    commands
        .spawn((
            DespawnOnExit(GameState::Loading),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(18),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
        ))
        .with_children(|screen| {
            for &spec in STAR_FIELD {
                screen.spawn(cosmic_background::star(spec));
            }

            screen.spawn(typography::title(
                localization.text(language, "loading.world").to_owned(),
            ));
            screen.spawn((
                LoadingSummaryText,
                typography::muted(
                    localization
                        .text(language, "loading.summary.pending")
                        .to_owned(),
                ),
            ));

            screen
                .spawn(surface::hud_container(Node {
                    width: px(LOADING_PANEL_WIDTH),
                    padding: UiRect::all(px(14)),
                    border: UiRect::all(px(1)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(LOADING_ROW_GAP),
                    ..default()
                }))
                .with_children(|phase_list| {
                    for (index, phase) in WorldLoadingPhase::ALL.into_iter().enumerate() {
                        spawn_loading_phase_row(
                            phase_list,
                            &localization,
                            language,
                            index + 1,
                            phase,
                        );
                    }
                });
        });
}

fn spawn_loading_phase_row(
    parent: &mut ChildSpawnerCommands,
    localization: &UiLocalization,
    language: Language,
    index: usize,
    phase: WorldLoadingPhase,
) {
    parent
        .spawn((
            LoadingPhaseRow(phase),
            Node {
                width: percent(100),
                height: px(LOADING_ROW_HEIGHT),
                padding: UiRect::horizontal(px(12)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            row.spawn((
                typography::caption(format!("{index:02}")),
                Node {
                    width: px(LOADING_PHASE_INDEX_WIDTH),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));

            row.spawn((
                LoadingPhaseLabel(phase),
                typography::hud(
                    localization
                        .text(language, loading_phase_key(phase))
                        .to_owned(),
                ),
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ));

            row.spawn((
                LoadingPhaseStatusText(phase),
                typography::caption(
                    localization
                        .text(language, "loading.status.pending")
                        .to_owned(),
                ),
                TextLayout::justify(Justify::Right),
                Node {
                    min_width: px(150),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

fn update_loading_progress(
    loading_state: Option<Res<WorldLoadingState>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut ui: LoadingProgressUi,
) {
    let Some(loading_state) = loading_state else {
        return;
    };
    let language = language.get();

    if let Ok(mut summary) = ui.summaries.single_mut() {
        let next = format_loading_summary(&localization, language, &loading_state);
        if **summary != next {
            **summary = next;
        }
    }

    for (phase_row, mut background) in &mut ui.rows {
        let status = loading_state.phase_status(phase_row.0);
        let target = if status == WorldLoadingPhaseStatus::Active {
            theme::PURPLE_SOFT
        } else {
            Color::NONE
        };
        if background.0 != target {
            background.0 = target;
        }
    }

    for (phase_label, mut color) in &mut ui.labels {
        let target = phase_text_color(loading_state.phase_status(phase_label.0));
        if color.0 != target {
            color.0 = target;
        }
    }

    for (status_label, mut text, mut color) in &mut ui.statuses {
        let phase = status_label.0;
        let status = loading_state.phase_status(phase);
        let next = format_loading_status(
            &localization,
            language,
            status,
            loading_state.phase_progress(phase),
        );
        if **text != next {
            **text = next;
        }

        let target = phase_status_color(status);
        if color.0 != target {
            color.0 = target;
        }
    }
}

fn format_loading_summary(
    localization: &UiLocalization,
    language: Language,
    loading_state: &WorldLoadingState,
) -> String {
    localization
        .text(language, "loading.summary")
        .replace("{columns}", &loading_state.column_count().to_string())
        .replace("{sections}", &loading_state.total().to_string())
}

fn format_loading_status(
    localization: &UiLocalization,
    language: Language,
    status: WorldLoadingPhaseStatus,
    progress: Option<(usize, usize)>,
) -> String {
    match status {
        WorldLoadingPhaseStatus::Pending => {
            localization.text(language, "loading.status.pending").to_owned()
        }
        WorldLoadingPhaseStatus::Active => progress
            .map(|(completed, total)| format!("{completed}/{total}"))
            .unwrap_or_default(),
        WorldLoadingPhaseStatus::Done => {
            localization.text(language, "loading.status.done").to_owned()
        }
    }
}

fn loading_phase_key(phase: WorldLoadingPhase) -> &'static str {
    match phase {
        WorldLoadingPhase::Generating => "loading.phase.generating",
        WorldLoadingPhase::SettlingFluids => "loading.phase.settlingFluids",
        WorldLoadingPhase::Lighting => "loading.phase.lighting",
        WorldLoadingPhase::Meshing => "loading.phase.meshing",
        WorldLoadingPhase::Assets => "loading.phase.assets",
        WorldLoadingPhase::Finalizing => "loading.phase.finalizing",
        WorldLoadingPhase::Spawning => "loading.phase.spawning",
    }
}

fn phase_text_color(status: WorldLoadingPhaseStatus) -> Color {
    match status {
        WorldLoadingPhaseStatus::Pending => theme::TEXT_SUBTLE,
        WorldLoadingPhaseStatus::Active | WorldLoadingPhaseStatus::Done => theme::TEXT_PRIMARY,
    }
}

fn phase_status_color(status: WorldLoadingPhaseStatus) -> Color {
    match status {
        WorldLoadingPhaseStatus::Pending => theme::TEXT_SUBTLE,
        WorldLoadingPhaseStatus::Active => theme::BORDER_FOCUS,
        WorldLoadingPhaseStatus::Done => theme::TEXT_MUTED,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_progress_system_initializes_without_query_conflicts() {
        let mut world = World::new();
        let mut system = IntoSystem::into_system(update_loading_progress);

        system.initialize(&mut world);
    }
}
