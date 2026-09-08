use bevy::prelude::*;

use crate::app::game_state::GameState;

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_crosshair);
    }
}

fn spawn_crosshair(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Text::new("+"),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}
