use bevy::prelude::*;

use crate::app::game_state::GameState;

use super::environment::EnvironmentVisualState;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_sky_color.run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn update_sky_color(visuals: Res<EnvironmentVisualState>, mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = visuals.sky_color;
}
