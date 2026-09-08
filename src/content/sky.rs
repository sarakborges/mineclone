use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{color::Rgb, day_night_phase::DayNightPhase};

#[derive(Clone, Deserialize)]
pub struct CelestialBodyDefinition {
    pub texture: Option<String>,
    pub size: f32,
    pub orbit_radius: f32,
    pub rise_phase: DayNightPhase,
    pub set_phase: DayNightPhase,
    pub rise_azimuth_degrees: f32,
    pub set_azimuth_degrees: f32,
    pub max_altitude_degrees: f32,
    pub light_fade_altitude_degrees: f32,
    pub tint: Rgb,
}

#[derive(Clone, Deserialize)]
pub struct SkyDefinition {
    pub id: String,
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
