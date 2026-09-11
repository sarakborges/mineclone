pub mod biome;
pub mod biome_density;
pub mod biome_distribution;
pub mod biome_hydrology;
pub mod biome_material;
pub mod biome_sky_layer;
pub mod biome_structure;
pub mod biome_terrain;
pub mod biome_terrain_modifier;
pub mod block;
pub(crate) mod block_id;
pub mod block_orientation;
pub(crate) mod builtin_ids;
pub mod color;
pub mod day_night_cycle;
pub mod day_night_phase;
pub mod dimension;
pub mod dimension_hydrology;
pub mod fluid;
mod json_file;
mod loader;
pub mod sky;
pub mod structure;
pub mod structure_set;

use bevy::prelude::*;
use loader::load_content;
pub(crate) use loader::read_content;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_content);
    }
}
