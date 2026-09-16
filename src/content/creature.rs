use std::path::{Component, Path};

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{color::Hsi, registry::DefinitionMap};

/// One reusable model can be referenced by any number of creature definitions.
/// Model paths are relative to `assets/`, never absolute filesystem paths.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatureDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub model: String,
    pub collider: CreatureCollider,
    #[serde(default)]
    pub material_tints: std::collections::HashMap<String, Hsi>,
    #[serde(default)]
    pub animations: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub preview_spawn: bool,
    #[serde(default = "default_jump_speed")]
    pub jump_speed: f32,
    #[serde(default = "default_jump_interval")]
    pub jump_interval: f32,
    #[serde(default)]
    pub anticipation_seconds: f32,
    #[serde(default)]
    pub landing_seconds: f32,
}

fn default_jump_speed() -> f32 {
    0.0
}

fn default_jump_interval() -> f32 {
    2.0
}

/// Kept on the creature root: glTF animation must never move or scale this AABB.
#[derive(Clone, Copy, Debug, Deserialize, Component)]
#[serde(rename_all = "camelCase")]
pub struct CreatureCollider {
    pub size: [f32; 3],
    pub center_offset: [f32; 3],
}

impl CreatureCollider {
    pub fn bounds(self, feet: Vec3) -> (Vec3, Vec3) {
        let center = feet + Vec3::from_array(self.center_offset);
        let half = Vec3::from_array(self.size) * 0.5;
        (center - half, center + half)
    }

    fn validate(self, id: &str) {
        assert!(self.size.iter().all(|value| value.is_finite() && *value > 0.0),
            "creature {id} collider size must be positive and finite");
        assert!(self.center_offset.iter().all(|value| value.is_finite()),
            "creature {id} collider offset must be finite");
    }
}

#[derive(Resource, Default)]
pub struct CreatureRegistry {
    definitions: DefinitionMap<CreatureDefinition>,
}

impl CreatureRegistry {
    pub fn insert(&mut self, definition: CreatureDefinition) {
        assert!(!definition.id.trim().is_empty(), "creature id cannot be empty");
        definition.name.validate(&format!("creature {} name", definition.id));
        assert!(valid_creature_model_path(&definition.model),
            "creature {} model must be a safe relative .glb/.gltf path under assets/models/creatures/: {}",
            definition.id, definition.model);
        definition.collider.validate(&definition.id);
        assert!(definition.jump_speed.is_finite() && definition.jump_speed >= 0.0,
            "creature {} has invalid jumpSpeed", definition.id);
        assert!(definition.jump_interval.is_finite() && definition.jump_interval > 0.0,
            "creature {} has invalid jumpInterval", definition.id);
        assert!(definition.anticipation_seconds.is_finite() && definition.anticipation_seconds >= 0.0,
            "creature {} has invalid anticipationSeconds", definition.id);
        assert!(definition.landing_seconds.is_finite() && definition.landing_seconds >= 0.0,
            "creature {} has invalid landingSeconds", definition.id);
        for (material, tint) in &definition.material_tints {
            assert!(!material.is_empty() && tint.is_valid(),
                "creature {} has an invalid material tint for {material}", definition.id);
        }
        for (state, clip) in &definition.animations {
            assert!(!state.is_empty() && !clip.is_empty(),
                "creature {} has an empty animation state/clip", definition.id);
        }
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&CreatureDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CreatureDefinition> {
        self.definitions.values()
    }
}

fn valid_creature_model_path(path: &str) -> bool {
    if path.contains('\\') || path.contains(':') || path.contains('\0') {
        return false;
    }
    let candidate = Path::new(path);
    if !candidate.components().all(|component| matches!(component, Component::Normal(_))) {
        return false;
    }
    let mut parts = path.split('/');
    if parts.next() != Some("models") || parts.next() != Some("creatures") {
        return false;
    }
    if !parts.all(|part| !part.is_empty() && part != "." && part != "..") {
        return false;
    }
    matches!(candidate.extension().and_then(|ext| ext.to_str()), Some("glb" | "gltf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_paths_remain_inside_moddable_creatures_directory() {
        assert!(valid_creature_model_path("models/creatures/slime/slime.glb"));
        assert!(!valid_creature_model_path("models/creatures/../secret.glb"));
        assert!(!valid_creature_model_path("/models/creatures/slime.glb"));
        assert!(!valid_creature_model_path("models/blocks/stone.glb"));
        assert!(!valid_creature_model_path("models/creatures\\secret.glb"));
    }

    #[test]
    fn collider_bounds_stay_independent_of_visual_animation() {
        let box_shape = CreatureCollider {
            size: [0.78, 0.84, 0.78],
            center_offset: [0.0, 0.42, 0.0],
        };
        let (min, max) = box_shape.bounds(Vec3::new(2.0, 3.0, 4.0));
        assert!((min - Vec3::new(1.61, 3.0, 3.61)).length() < 0.0001);
        assert!((max - Vec3::new(2.39, 3.84, 4.39)).length() < 0.0001);
    }
}