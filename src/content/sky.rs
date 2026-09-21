use bevy::prelude::*;
use serde::Deserialize;

use super::{color::Hsi, day_night_phase::DayNightPhase, registry::DefinitionMap};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CelestialBodyDefinition {
    pub texture: Option<String>,
    pub size: f32,
    pub orbit_radius: f32,
    pub rise_phase: DayNightPhase,
    pub set_phase: DayNightPhase,
    pub rise_azimuth_degrees: f32,
    pub set_azimuth_degrees: f32,
    pub max_altitude_degrees: f32,
    pub tint: Hsi,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkyDefinition {
    pub id: String,
    pub sun: CelestialBodyDefinition,
    pub moon: CelestialBodyDefinition,
}

#[derive(Resource, Default)]
pub struct SkyRegistry {
    definitions: DefinitionMap<SkyDefinition>,
}

impl SkyRegistry {
    pub fn insert(&mut self, definition: SkyDefinition) {
        assert!(!definition.id.trim().is_empty(), "sky id cannot be empty");
        validate_celestial_body(&definition.id, "sun", &definition.sun);
        validate_celestial_body(&definition.id, "moon", &definition.moon);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&SkyDefinition> {
        self.definitions.get(id)
    }
}

fn validate_celestial_body(sky_id: &str, body_name: &str, body: &CelestialBodyDefinition) {
    assert!(
        body.size.is_finite() && body.size > 0.0,
        "sky {sky_id} {body_name} size must be positive and finite"
    );
    assert!(
        body.orbit_radius.is_finite() && body.orbit_radius > 0.0,
        "sky {sky_id} {body_name} orbit radius must be positive and finite"
    );
    for (field, value) in [
        ("riseAzimuthDegrees", body.rise_azimuth_degrees),
        ("setAzimuthDegrees", body.set_azimuth_degrees),
        ("maxAltitudeDegrees", body.max_altitude_degrees),
    ] {
        assert!(
            value.is_finite(),
            "sky {sky_id} {body_name} {field} must be finite"
        );
    }
    assert!(
        body.tint.is_valid(),
        "sky {sky_id} {body_name} HSI tint is invalid"
    );
}
