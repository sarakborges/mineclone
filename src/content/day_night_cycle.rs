use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::color::Rgb;

#[derive(Clone, Copy, Deserialize)]
pub struct DayNightKeyframe {
    pub time: f32,
    pub sky_tint: Rgb,
    pub fog_tint: Rgb,
    pub ambient_color: Rgb,
    pub ambient_brightness: f32,
    pub sun_color: Rgb,
    pub sun_illuminance: f32,
}

#[derive(Clone, Deserialize)]
pub struct DayNightCycleDefinition {
    pub id: String,
    pub duration_seconds: f32,
    pub initial_time: f32,
    pub sun_angle_offset_degrees: f32,
    pub keyframes: Vec<DayNightKeyframe>,
}

#[derive(Resource, Default)]
pub struct DayNightCycleRegistry {
    definitions: HashMap<String, DayNightCycleDefinition>,
}

impl DayNightCycleRegistry {
    pub fn insert(&mut self, definition: DayNightCycleDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&DayNightCycleDefinition> {
        self.definitions.get(id)
    }
}
