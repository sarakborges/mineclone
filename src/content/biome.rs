use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

use super::biome_density::BiomeDensityModifier;
use crate::rendering::sky_layers::SkyLayerDefinition;

mod validation;
use validation::validate_biome_definition;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BiomeClimate {
    pub temperature: f32,
    pub humidity: f32,
    pub continentalness: f32,
    pub erosion: f32,
    pub weirdness: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BiomeRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BiomeSize {
    pub x: BiomeRange,
    pub y: Option<BiomeRange>,
    pub z: BiomeRange,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BiomeVerticalRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum BiomeKind {
    Surface,
    Volume,
    Hydrology,
}

#[derive(Clone, Copy, Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BiomeHydrology {
    #[serde(default = "default_hydrology_multiplier")]
    pub river_width_multiplier: f32,
    #[serde(default = "default_hydrology_multiplier")]
    pub river_depth_multiplier: f32,
    #[serde(default = "default_hydrology_multiplier")]
    pub lake_size_multiplier: f32,
}

const fn default_hydrology_multiplier() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeVisuals {
    pub sky_color: Srgba,
    pub fog_color: Srgba,
    pub fog_density: f32,
    pub grass_color: Srgba,
    pub water_color: Srgba,
    pub ambient_light: f32,
    #[serde(default)]
    pub sky_layers: Vec<SkyLayerDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeDefinition {
    pub id: String,
    pub name: String,
    pub kind: BiomeKind,
    pub size: BiomeSize,
    pub climate: BiomeClimate,
    #[serde(default)]
    pub vertical_range: Option<BiomeVerticalRange>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub density_modifier: Option<BiomeDensityModifier>,
    #[serde(default)]
    pub solid_block: Option<String>,
    #[serde(default)]
    pub hydrology: BiomeHydrology,
    pub visuals: BiomeVisuals,
}

#[derive(Resource, Default)]
pub struct BiomeRegistry {
    definitions: HashMap<String, BiomeDefinition>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        validate_biome_definition(&definition);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
        self.definitions.get(id)
    }

    pub fn has_volume_density_modifiers(&self) -> bool {
        self.definitions.values().any(|definition| {
            definition.kind == BiomeKind::Volume && definition.density_modifier.is_some()
        })
    }
}
