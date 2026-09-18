use bevy::prelude::*;
use serde::Deserialize;

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
}

fn default_player_health() -> f32 { 20.0 }
fn default_player_attack() -> String { "asteria:punch".to_owned() }

impl PlayerDefinition {
    pub fn validate(&self) {
        assert!(self.health.is_finite() && self.health > 0.0, "player health must be positive and finite");
        assert!(!self.attack.trim().is_empty(), "player attack must not be empty");
        if let Some(model) = &self.model {
            assert!(
                !model.is_empty() && !model.contains('\\') && !model.contains(':') && !model.contains('\0'),
                "player model path must be a safe relative path: {model}"
            );
        }
    }
}
