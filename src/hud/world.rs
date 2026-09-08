mod banner;
mod biome;
mod coordinates;
mod dimension;
mod layout;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use biome::update_biome_hud;
use coordinates::update_coordinates_hud;
use dimension::update_dimension_hud;
use layout::spawn_world_hud;

pub struct WorldHudPlugin;

impl Plugin for WorldHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_world_hud)
            .add_systems(
                Update,
                (
                    update_dimension_hud,
                    update_biome_hud,
                    update_coordinates_hud,
                )
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}
