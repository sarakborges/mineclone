mod validation;

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use self::validation::validate_biome_definition;
use super::{
    biome_density::BiomeDensityModifier, biome_distribution::BiomeDistribution,
    biome_hydrology::BiomeHydrology, biome_material::BiomeMaterialLayer,
    biome_sky_layer::BiomeSkyLayerVisuals, biome_structure::BiomeStructure,
    biome_terrain::BiomeTerrain, biome_terrain_modifier::BiomeTerrainModifier, color::Rgb,
    day_night_phase::DayNightPhases,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BiomeKind {
    #[default]
    Surface,
    Volume,
    Hydrology,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeSizeAxis {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeSize {
    pub x: BiomeSizeAxis,
    pub z: BiomeSizeAxis,
    #[serde(default)]
    pub y: Option<BiomeSizeAxis>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeClimateRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeClimate {
    #[serde(default)]
    pub temperature: Option<BiomeClimateRange>,
    #[serde(default)]
    pub humidity: Option<BiomeClimateRange>,
    #[serde(default)]
    pub continentalness: Option<BiomeClimateRange>,
    #[serde(default)]
    pub erosion: Option<BiomeClimateRange>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeVerticalRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeUnderwaterTint {
    pub color: Rgb,
    pub opacity: f32,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeVisuals {
    pub sky_color: DayNightPhases<Rgb>,
    pub fog_color: DayNightPhases<Rgb>,
    #[serde(default = "default_vegetation_color")]
    pub grass_color: Rgb,
    #[serde(default = "default_vegetation_color")]
    pub leaf_color: Rgb,
    #[serde(default = "default_vegetation_color")]
    pub foliage_color: Rgb,
    pub underwater_tint: BiomeUnderwaterTint,
    #[serde(default)]
    pub stars: BiomeSkyLayerVisuals,
    #[serde(default)]
    pub clouds: BiomeSkyLayerVisuals,
    pub terrain_roughness: f32,
    pub terrain_metallic: f32,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: BiomeKind,
    #[serde(default = "default_biome_distributions")]
    pub distributions: Vec<BiomeDistribution>,
    #[serde(default)]
    pub size: BiomeSize,
    #[serde(default)]
    pub climate: BiomeClimate,
    #[serde(default)]
    pub vertical_range: Option<BiomeVerticalRange>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub terrain: Option<BiomeTerrain>,
    #[serde(default)]
    pub terrain_modifiers: Vec<BiomeTerrainModifier>,
    #[serde(default)]
    pub surface_layers: Vec<BiomeMaterialLayer>,
    #[serde(default)]
    pub density_modifier: Option<BiomeDensityModifier>,
    #[serde(default)]
    pub solid_block: Option<String>,
    #[serde(default)]
    pub hydrology: BiomeHydrology,
    #[serde(default)]
    pub structures: Vec<BiomeStructure>,
    #[serde(default)]
    pub structure_sets: Vec<String>,
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

    pub fn iter(&self) -> impl Iterator<Item = &BiomeDefinition> {
        self.definitions.values()
    }

    pub fn has_volume_density_modifiers(&self) -> bool {
        self.definitions.values().any(|definition| {
            definition.kind == BiomeKind::Volume && definition.density_modifier.is_some()
        })
    }
}

fn default_biome_distributions() -> Vec<BiomeDistribution> {
    vec![BiomeDistribution::Regional]
}

fn default_vegetation_color() -> Rgb {
    Rgb {
        r: 0.24,
        g: 0.70,
        b: 0.16,
    }
}
