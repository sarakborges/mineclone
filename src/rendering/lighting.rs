use bevy::prelude::*;

use crate::app::game_state::GameState;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), setup_lighting);
    }
}

fn setup_lighting(mut ambient_light: ResMut<GlobalAmbientLight>) {
    ambient_light.color = Color::srgb(0.9, 0.94, 1.0);
    ambient_light.brightness = 450.0;
}
