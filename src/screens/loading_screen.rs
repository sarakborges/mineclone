use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
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

fn setup_loading_screen(mut commands: Commands) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::Loading)));

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
            typography::heading("Loading world..."),
            (
                typography::muted("0 out of 0 chunks generated"),
                LoadingProgressText,
            ),
        ],
    ));
}

fn update_loading_progress(
    loading_state: Option<Res<WorldLoadingState>>,
    mut labels: Query<&mut Text, With<LoadingProgressText>>,
) {
    let Some(loading_state) = loading_state else {
        return;
    };

    let Ok(mut label) = labels.single_mut() else {
        return;
    };

    **label = format!(
        "{} out of {} chunks generated",
        loading_state.generated(),
        loading_state.total(),
    );
}
