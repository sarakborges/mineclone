pub(crate) mod render_distance;
mod setup;
mod test_world;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use setup::setup_world;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), setup_world);
    }
}
