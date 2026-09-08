use bevy::prelude::*;

use super::camera::GameplayCamera;
use crate::app::game_state::GameState;

pub struct PlayerHudPlugin;

impl Plugin for PlayerHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_player_hud)
            .add_systems(Update, update_player_hud.run_if(in_state(GameState::Gameplay)));
    }
}

#[derive(Component)]
struct PlayerCoordinatesText;

fn spawn_player_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.82)),
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Text::new("X: 0.00\nZ: 0.00\nY: 0.00"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            PlayerCoordinatesText,
        )],
    ));
}

fn update_player_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    mut coordinates_text: Single<&mut Text, With<PlayerCoordinatesText>>,
) {
    let position = player.translation;

    coordinates_text.0 = format!(
        "X: {:.2}\nZ: {:.2}\nY: {:.2}",
        position.x, position.z, position.y
    );
}
