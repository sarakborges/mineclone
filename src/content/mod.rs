pub(crate) mod biome;
pub(crate) mod biome_density;
pub(crate) mod biome_distribution;
pub(crate) mod biome_hydrology;
pub(crate) mod biome_material;
pub(crate) mod biome_sky_layer;
pub(crate) mod biome_structure;
pub(crate) mod biome_surface_carver;
pub(crate) mod biome_terrain;
pub(crate) mod biome_terrain_modifier;
pub(crate) mod block;
pub(crate) mod block_id;
pub(crate) mod block_orientation;
pub(crate) mod builtin_ids;
pub(crate) mod color;
pub(crate) mod day_night_cycle;
pub(crate) mod day_night_phase;
pub(crate) mod dimension;
pub(crate) mod dimension_hydrology;
pub(crate) mod fluid;
pub(crate) mod inventory_category;
mod json_file;
mod loader;
mod registry;
pub(crate) mod sky;
pub(crate) mod structure;

use bevy::prelude::*;
use loader::load_content;
pub(crate) use loader::read_content;

pub(crate) struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_content);
    }
}
