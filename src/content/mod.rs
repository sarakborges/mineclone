pub(crate) mod ambient_particle;
pub(crate) mod ambient_particle_registry;
pub(crate) mod asset_path;
pub(crate) mod attack;
// Phase 1 intentionally disconnects authored world-generation content from its
// runtime consumers. Keep these forward definitions intact until the replacement
// generator owns them again instead of deleting fields merely to satisfy lints.
#[allow(dead_code)]
pub(crate) mod biome;
pub(crate) mod biome_density;
pub(crate) mod biome_distribution;
#[allow(dead_code)]
pub(crate) mod biome_material;
pub(crate) mod biome_sky_layer;
#[allow(dead_code)]
pub(crate) mod biome_structure;
pub(crate) mod biome_surface_fluid;
#[allow(dead_code)]
pub(crate) mod biome_surface_margin;
#[allow(dead_code)]
pub(crate) mod biome_terrain;
#[allow(dead_code)]
pub(crate) mod biome_terrain_modifier;
pub(crate) mod block;
pub(crate) mod block_id;
pub(crate) mod block_orientation;
pub(crate) mod block_shape;
pub(crate) mod builtin_ids;
pub(crate) mod color;
pub(crate) mod crafting_recipe;
pub(crate) mod creature;
pub(crate) mod day_night_cycle;
pub(crate) mod day_night_phase;
pub(crate) mod dimension;
pub(crate) mod fluid;
pub(crate) mod inventory_category;
pub(crate) mod item;
pub(crate) mod item_id;
mod json_file;
pub(crate) mod layer;
pub(crate) mod layer_id;
mod loader;
pub(crate) mod loot;
pub(crate) mod object;
pub(crate) mod object_id;
pub(crate) mod player;
pub(crate) mod portal;
mod registry;
pub(crate) mod secondary_property;
pub(crate) mod sky;
#[allow(dead_code)]
pub(crate) mod structure;
#[allow(dead_code)]
pub(crate) mod structure_rules;
#[allow(dead_code)]
pub(crate) mod structure_set;
pub(crate) mod tool;
pub(crate) mod tool_behavior;
pub(crate) mod tool_category;
pub(crate) mod tool_id;
mod validation;
pub(crate) mod world_recipe;

use bevy::prelude::*;
use loader::load_content;

pub(crate) struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_content);
    }
}
