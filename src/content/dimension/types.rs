use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use crate::content::{
    builtin_ids::WATER_FLUID_ID,
    registry::DefinitionMap,
};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionBiomeSizeAxis {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionBiomeSize {
    pub x: DimensionBiomeSizeAxis,
    pub z: DimensionBiomeSizeAxis,
    #[serde(default)]
    pub y: Option<DimensionBiomeSizeAxis>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionBiome {
    pub id: String,
    #[serde(default = "default_biome_weight")]
    pub weight: f32,
    #[serde(default = "default_spawn_biome_weight")]
    pub spawn_weight: f32,
    #[serde(default)]
    pub size: Option<DimensionBiomeSize>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub biomes: Vec<DimensionBiome>,
    pub day_night_cycle: String,
    pub sky: String,
    pub sea_level: i32,
    pub gravity_strength: f32,
    #[serde(default = "default_sea_fluid")]
    pub sea_fluid: String,
    #[serde(default)]
    pub ocean_biome: Option<String>,
    #[serde(default = "default_max_entities")]
    pub max_entities: usize,
}

fn default_max_entities() -> usize { 128 }
fn default_spawn_biome_weight() -> f32 { 1.0 }
fn default_biome_weight() -> f32 { 1.0 }
fn default_sea_fluid() -> String { WATER_FLUID_ID.to_owned() }

#[derive(Resource, Default)]
pub struct DimensionRegistry {
    definitions: DefinitionMap<DimensionDefinition>,
}

impl DimensionRegistry {
    pub fn insert(&mut self, definition: DimensionDefinition) {
        definition.name.validate(&format!("dimension {} name", definition.id));
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DimensionDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DimensionDefinition> {
        self.definitions.values()
    }
}
