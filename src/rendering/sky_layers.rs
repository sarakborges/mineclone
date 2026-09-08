mod clouds;
mod stars;
mod state;

use bevy::prelude::*;

use crate::app::game_state::GameState;
use clouds::{setup_cloud_assets, spawn_clouds, update_clouds};
use stars::{setup_star_assets, spawn_stars, update_stars};
use state::{update_sky_layer_visuals, SkyLayerVisualState};

pub struct SkyLayersPlugin;

impl Plugin for SkyLayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkyLayerVisualState>()
            .add_systems(Startup, (setup_star_assets, setup_cloud_assets))
            .add_systems(OnEnter(GameState::Gameplay), (spawn_stars, spawn_clouds))
            .add_systems(
                Update,
                update_sky_layer_visuals.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                PostUpdate,
                (update_stars, update_clouds).run_if(in_state(GameState::Gameplay)),
            );
    }
}
