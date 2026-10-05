use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, UiLocalization},
    ui::{theme, typography},
    world::loading::{WorldLoadingPhase, WorldLoadingProgress},
};

pub(super) struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), setup_loading_screen)
            .add_systems(
                Update,
                sync_loading_screen.run_if(in_state(GameState::Loading)),
            );
    }
}

#[derive(Component)]
struct LoadingTitle;

#[derive(Component)]
struct LoadingPhaseLabel;

#[derive(Component)]
struct LoadingProgressLabel;

#[derive(Component)]
struct LoadingProgressFill;

type LoadingTitleQuery<'w, 's> = Query<'w, 's, &'static mut Text, With<LoadingTitle>>;
type LoadingPhaseQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (With<LoadingPhaseLabel>, Without<LoadingTitle>),
>;
type LoadingProgressLabelQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<LoadingProgressLabel>,
        Without<LoadingTitle>,
        Without<LoadingPhaseLabel>,
    ),
>;
type LoadingProgressFillQuery<'w, 's> =
    Query<'w, 's, &'static mut Node, With<LoadingProgressFill>>;

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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(560),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(18),
                    padding: UiRect::all(px(28)),
                    ..default()
                },
                BackgroundColor(theme::FROSTED_SURFACE),
                theme::frosted_surface_gradient(),
            ))
            .with_children(|panel| {
                panel.spawn((
                    typography::heading(localization.text(language, "loading.world")),
                    LoadingTitle,
                ));
                panel.spawn((
                    typography::muted(localization.text(language, "loading.summary.pending")),
                    LoadingPhaseLabel,
                ));
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            height: px(12),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(theme::SURFACE_INSET),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            Node {
                                width: percent(0),
                                height: percent(100),
                                ..default()
                            },
                            BackgroundColor(theme::PURPLE),
                            LoadingProgressFill,
                        ));
                    });
                panel.spawn((typography::caption("0 / 0"), LoadingProgressLabel));
            });
        });
}

fn sync_loading_screen(
    progress: Res<WorldLoadingProgress>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut titles: LoadingTitleQuery,
    mut phases: LoadingPhaseQuery,
    mut labels: LoadingProgressLabelQuery,
    mut fills: LoadingProgressFillQuery,
) {
    if !progress.is_changed() && !localization.is_changed() && !language.is_changed() {
        return;
    }

    let language = language.get();
    let phase_key = match progress.phase() {
        WorldLoadingPhase::PreparingDestination => "loading.summary.pending",
        WorldLoadingPhase::MaterializingInitialArea => "loading.phase.generating",
        WorldLoadingPhase::Ready => "loading.status.done",
    };

    for mut title in &mut titles {
        title.0 = localization.text(language, "loading.world").to_owned();
    }
    for mut phase in &mut phases {
        phase.0 = localization.text(language, phase_key).to_owned();
    }

    let completed = progress.completed();
    let total = progress.total();
    for mut label in &mut labels {
        label.0 = if total == 0 {
            localization
                .text(language, "loading.summary.pending")
                .to_owned()
        } else {
            format!("{completed} / {total}")
        };
    }

    let percentage = if total == 0 {
        0.0
    } else {
        (completed as f32 / total as f32).clamp(0.0, 1.0) * 100.0
    };
    for mut fill in &mut fills {
        fill.width = percent(percentage);
    }
}
