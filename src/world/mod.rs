pub mod biome;
pub mod biome_field;
pub mod day_night;
pub mod dimension;
pub(crate) mod render_distance;
mod setup;
mod test_world;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use biome::{track_current_biome, CurrentBiome};
use day_night::DayNightPlugin;
use dimension::CurrentDimension;
use setup::setup_world;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDimension>()
            .init_resource::<CurrentBiome>()
            .add_plugins(DayNightPlugin)
            .add_systems(OnEnter(GameState::Gameplay), setup_world)
            .add_systems(
                Update,
                track_current_biome.run_if(in_state(GameState::Gameplay)),
            );
    }
}
