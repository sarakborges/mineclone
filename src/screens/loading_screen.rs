use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{theme, typography},
    world::{WorldLoadingPhase, WorldLoadingPhaseStatus, WorldLoadingState},
};

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
struct LoadingPhaseText(WorldLoadingPhase);

fn setup_loading_screen(
    mut commands: Commands,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::Loading)));
    let language = language.get();

    commands
        .spawn((
            DespawnOnExit(GameState::Loading),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(16),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
        ))
        .with_children(|screen| {
            screen.spawn(typography::heading(
                localization.text(language, "loading.world").to_owned(),
            ));
            screen
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(6),
                    min_width: px(420),
                    ..default()
                })
                .with_children(|phase_list| {
                    for phase in WorldLoadingPhase::ALL {
                        let text = format_loading_phase(
                            &localization,
                            language,
                            phase,
                            WorldLoadingPhaseStatus::Pending,
                            None,
                        );
                        phase_list.spawn((typography::muted(text), LoadingPhaseText(phase)));
                    }
                });
        });
}

fn update_loading_progress(
    loading_state: Option<Res<WorldLoadingState>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut labels: Query<(&LoadingPhaseText, &mut Text)>,
) {
    let Some(loading_state) = loading_state else {
        return;
    };

    for (phase_label, mut text) in &mut labels {
        let phase = phase_label.0;
        let next = format_loading_phase(
            &localization,
            language.get(),
            phase,
            loading_state.phase_status(phase),
            loading_state.phase_progress(phase),
        );
        if **text != next {
            **text = next;
        }
    }
}

fn format_loading_phase(
    localization: &UiLocalization,
    language: Language,
    phase: WorldLoadingPhase,
    status: WorldLoadingPhaseStatus,
    progress: Option<(usize, usize)>,
) -> String {
    let phase_key = match phase {
        WorldLoadingPhase::Generating => "loading.phase.generating",
        WorldLoadingPhase::SettlingFluids => "loading.phase.settlingFluids",
        WorldLoadingPhase::Lighting => "loading.phase.lighting",
        WorldLoadingPhase::Meshing => "loading.phase.meshing",
        WorldLoadingPhase::Assets => "loading.phase.assets",
        WorldLoadingPhase::Finalizing => "loading.phase.finalizing",
        WorldLoadingPhase::Spawning => "loading.phase.spawning",
    };
    let status_key = match status {
        WorldLoadingPhaseStatus::Pending => "loading.status.pending",
        WorldLoadingPhaseStatus::Active => "loading.status.active",
        WorldLoadingPhaseStatus::Done => "loading.status.done",
    };

    let status = localization.text(language, status_key);
    let phase = localization.text(language, phase_key);
    progress.map_or_else(
        || format!("[{status}] {phase}"),
        |(completed, total)| format!("[{status}] {phase} · {completed}/{total}"),
    )
}
