use bevy::prelude::*;

use crate::game_state::GameState;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), setup_gameplay);
    }
}

fn setup_gameplay(mut commands: Commands) {
    commands.spawn((Camera3d::default(), DespawnOnExit(GameState::Gameplay)));
}
