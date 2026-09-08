pub mod biome;
pub mod dimension;
pub(crate) mod render_distance;
mod setup;
mod test_world;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use biome::CurrentBiome;
use dimension::CurrentDimension;
use setup::setup_world;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDimension>()
            .init_resource::<CurrentBiome>()
            .add_systems(OnEnter(GameState::Gameplay), setup_world);
    }
}
