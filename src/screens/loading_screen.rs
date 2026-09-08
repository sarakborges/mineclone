use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    ui::{theme, typography},
};

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), setup_loading_screen);
    }
}

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
            typography::muted("Generating terrain"),
        ],
    ));
}
