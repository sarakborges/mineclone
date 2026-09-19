mod banner;
mod coordinates;
mod layout;
mod named;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, dimension::DimensionRegistry},
    world::{biome::CurrentBiome, dimension::CurrentDimension},
};
use coordinates::update_coordinates_hud;
use layout::spawn_world_hud;
use named::{BiomeHudText, DimensionHudText, update_localized_name_hud};

pub struct WorldHudPlugin;

impl Plugin for WorldHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_world_hud)
            .add_systems(
                Update,
                (
                    update_localized_name_hud::<
                        CurrentDimension,
                        DimensionRegistry,
                        DimensionHudText,
                    >,
                    update_localized_name_hud::<CurrentBiome, BiomeRegistry, BiomeHudText>,
                    update_coordinates_hud,
                )
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}
