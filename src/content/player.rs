use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{asset_path::is_safe_relative_asset_path, attack::AttackRegistry};

/// Data-driven definition for the local player entity.
#[derive(Resource, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDefinition {
    #[serde(default = "default_player_health")]
    pub health: f32,
    #[serde(default = "default_player_attack")]
    pub attack: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub animations: HashMap<String, String>,
}

fn default_player_health() -> f32 { 20.0 }
fn default_player_attack() -> String { "asteria:punch".to_owned() }

impl PlayerDefinition {
    pub fn validate(&self) {
        assert!(self.health.is_finite() && self.health > 0.0, "player health must be positive and finite");
        assert!(!self.attack.trim().is_empty(), "player attack must not be empty");
        if let Some(model) = &self.model {
            assert!(
                is_safe_relative_asset_path(model),
                "player model path must be a safe relative asset path: {model}"
            );
        }
        for (state, clip) in &self.animations {
            assert!(
                !state.trim().is_empty() && !clip.trim().is_empty(),
                "player animation mappings must use non-empty state and clip names"
            );
        }
    }

    pub(crate) fn validate_references(&self, attacks: &AttackRegistry) {
        assert!(
            attacks.get(&self.attack).is_some(),
            "player references missing attack {}",
            self.attack
        );
    }
}
