pub mod biome;
pub mod biome_density;
pub mod biome_hydrology;
pub mod biome_sky_layer;
pub mod biome_terrain;
pub mod block;
pub mod color;
pub mod day_night_cycle;
pub mod day_night_phase;
pub mod dimension;
pub mod dimension_hydrology;
pub mod fluid;
pub mod sky;
mod json_file;
mod loader;

use bevy::prelude::*;
use loader::load_content;
pub(crate) use loader::read_content;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_content);
    }
}
