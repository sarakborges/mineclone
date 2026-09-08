use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{color::Rgb, day_night_phase::DayNightPhases};

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
    pub terrain_roughness: f32,
    pub terrain_metallic: f32,
}

#[derive(Clone, Deserialize)]
pub struct BiomeDefinition {
    pub id: String,
    pub name: String,
    pub size: BiomeSize,
    pub visuals: BiomeVisuals,
}

#[derive(Resource, Default)]
pub struct BiomeRegistry {
    definitions: HashMap<String, BiomeDefinition>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
        self.definitions.get(id)
    }
}
