use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{theme, typography},
    world::WorldLoadingState,
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
struct LoadingProgressText;

fn setup_loading_screen(
    mut commands: Commands,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::Loading)));
    let language = language.get();
    let progress = format_loading_progress(&localization, language, 0, 0);

    commands.spawn((
        DespawnOnExit(GameState::Loading),
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(12),
            ..default()
        },
        BackgroundColor(theme::SCREEN_BACKGROUND),
        theme::cosmic_background_gradient(),
        children![
            typography::heading(localization.text(language, "loading.world").to_owned()),
            (typography::muted(progress), LoadingProgressText),
        ],
    ));
}

fn update_loading_progress(
    loading_state: Option<Res<WorldLoadingState>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut labels: Query<&mut Text, With<LoadingProgressText>>,
) {
    let Some(loading_state) = loading_state else {
        return;
    };

    let Ok(mut label) = labels.single_mut() else {
        return;
    };

    **label = format_loading_progress(
        &localization,
        language.get(),
        loading_state.generated(),
        loading_state.total(),
    );
}

fn format_loading_progress(
    localization: &UiLocalization,
    language: Language,
    generated: usize,
    total: usize,
) -> String {
    localization
        .text(language, "loading.progress")
        .replace("{generated}", &generated.to_string())
        .replace("{total}", &total.to_string())
}
