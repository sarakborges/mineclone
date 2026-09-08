use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{
    biome_sky_layer::BiomeSkyLayerVisuals,
    biome_terrain::BiomeTerrain,
    color::Rgb,
    day_night_phase::DayNightPhases,
};

#[derive(Clone, Copy, Deserialize)]
pub struct BiomeSizeAxis {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Deserialize)]
pub struct BiomeSize {
    pub x: BiomeSizeAxis,
    pub z: BiomeSizeAxis,
    #[serde(default)]
    pub y: Option<BiomeSizeAxis>,
}

#[derive(Clone, Deserialize)]
pub struct BiomeVisuals {
    pub sky_color: DayNightPhases<Rgb>,
    pub fog_color: DayNightPhases<Rgb>,
    pub grass_color: Rgb,
    #[serde(default)]
    pub stars: BiomeSkyLayerVisuals,
    #[serde(default)]
    pub clouds: BiomeSkyLayerVisuals,
    pub terrain_roughness: f32,
    pub terrain_metallic: f32,
}

#[derive(Clone, Deserialize)]
pub struct BiomeDefinition {
    pub id: String,
    pub name: String,
    pub size: BiomeSize,
    pub terrain: BiomeTerrain,
    pub visuals: BiomeVisuals,
}

#[derive(Resource, Default)]
pub struct BiomeRegistry {
    definitions: HashMap<String, BiomeDefinition>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        assert!(
            (0.0..=1.0).contains(&definition.visuals.stars.density),
            "biome {} stars density must be between 0 and 1",
            definition.id
        );
        assert!(
            (0.0..=1.0).contains(&definition.visuals.clouds.density),
            "biome {} clouds density must be between 0 and 1",
            definition.id
        );
        definition.terrain.validate(&definition.id);

        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
        self.definitions.get(id)
    }
}
