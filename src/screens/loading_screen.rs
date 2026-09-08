use bevy::prelude::*;

use crate::app::game_state::GameState;

const BACKGROUND_COLOR: Color = Color::srgb(0.055, 0.065, 0.08);
const TEXT_COLOR: Color = Color::srgb(0.92, 0.94, 0.97);
const SUBTEXT_COLOR: Color = Color::srgb(0.62, 0.68, 0.78);

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
        BackgroundColor(BACKGROUND_COLOR),
        children![
            (
                Text::new("Loading world..."),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ),
            (
                Text::new("Generating terrain"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(SUBTEXT_COLOR),
            ),
        ],
    ));
}
