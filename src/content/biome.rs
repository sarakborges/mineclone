use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{
    color::Rgb,
    day_night_phase::DayNightPhases,
};

#[derive(Clone, Deserialize)]
pub struct BiomeVisuals {
    pub sky_color: DayNightPhases<Rgb>,
    pub fog_color: DayNightPhases<Rgb>,
    pub terrain_color: Rgb,
    pub terrain_roughness: f32,
    pub terrain_metallic: f32,
}

#[derive(Clone, Deserialize)]
pub struct BiomeDefinition {
    pub id: String,
    pub name: String,
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
