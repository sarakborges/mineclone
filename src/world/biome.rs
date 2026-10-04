use bevy::prelude::*;

use crate::content::builtin_ids::PLAINS_BIOME_ID;

pub const DEFAULT_BIOME_ID: &str = PLAINS_BIOME_ID;

#[derive(Clone, PartialEq)]
pub struct CurrentBiomeInfluence {
    pub id: String,
    pub weight: f32,
}

/// Runtime-facing biome presentation state retained while the authoritative
/// biome query owner is rebuilt. Phase 3 will repopulate this from the new
/// surface/volume biome capabilities.
#[derive(Resource, PartialEq)]
pub struct CurrentBiome {
    pub id: String,
    pub influences: Vec<CurrentBiomeInfluence>,
    pub surface_id: String,
    pub surface_influences: Vec<CurrentBiomeInfluence>,
    pub volume_id: Option<String>,
    pub volume_influences: Vec<CurrentBiomeInfluence>,
    pub volume_strength: f32,
}

impl Default for CurrentBiome {
    fn default() -> Self {
        let default_influence = CurrentBiomeInfluence {
            id: DEFAULT_BIOME_ID.to_owned(),
            weight: 1.0,
        };

        Self {
            id: DEFAULT_BIOME_ID.to_owned(),
            influences: vec![default_influence.clone()],
            surface_id: DEFAULT_BIOME_ID.to_owned(),
            surface_influences: vec![default_influence],
            volume_id: None,
            volume_influences: Vec::new(),
            volume_strength: 0.0,
        }
    }
}
