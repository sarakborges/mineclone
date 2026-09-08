use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::color::Rgb;

#[derive(Clone, Deserialize)]
pub struct CelestialBodyDefinition {
    pub texture: String,
    pub size: f32,
    pub orbit_radius: f32,
    pub phase_offset_degrees: f32,
    pub tint: Rgb,
}

#[derive(Clone, Deserialize)]
pub struct SkyDefinition {
    pub id: String,
    pub orbit_tilt_degrees: f32,
    pub sun: CelestialBodyDefinition,
    pub moon: CelestialBodyDefinition,
}

#[derive(Resource, Default)]
pub struct SkyRegistry {
    definitions: HashMap<String, SkyDefinition>,
}

impl SkyRegistry {
    pub fn insert(&mut self, definition: SkyDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&SkyDefinition> {
        self.definitions.get(id)
    }
}
