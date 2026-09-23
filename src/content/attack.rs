use bevy::prelude::*;
use serde::Deserialize;

use super::registry::DefinitionMap;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AttackEffectKind {
    Knockback,
}

impl AttackEffectKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Knockback => "knockback",
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackEffectDefinition {
    pub effect: AttackEffectKind,
    #[serde(default = "default_effect_chance")]
    pub chance: f32,
    #[serde(default)]
    pub strength: f32,
}

fn default_effect_chance() -> f32 { 1.0 }

impl AttackEffectDefinition {
    fn validate(&self, attack_id: &str) {
        let effect = self.effect.as_str();
        assert!(self.chance.is_finite() && (0.0..=1.0).contains(&self.chance), "attack {attack_id} effect {effect} chance must be between 0 and 1");
        assert!(self.strength.is_finite() && self.strength >= 0.0, "attack {attack_id} effect {effect} strength must be non-negative");
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackDefinition {
    pub id: String,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub effects: Vec<AttackEffectDefinition>,
}

impl AttackDefinition {
    pub fn validate(&self) {
        assert!(!self.id.trim().is_empty(), "attack id must not be empty");
        assert!(self.damage.is_finite() && self.damage >= 0.0, "attack {} damage must be non-negative", self.id);
        for effect in &self.effects {
            effect.validate(&self.id);
        }
    }
}

#[derive(Resource, Default)]
pub struct AttackRegistry {
    definitions: DefinitionMap<AttackDefinition>,
}

impl AttackRegistry {
    pub fn insert(&mut self, definition: AttackDefinition) {
        definition.validate();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&AttackDefinition> {
        self.definitions.get(id)
    }
}
