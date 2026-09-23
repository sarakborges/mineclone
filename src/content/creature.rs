use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{asset_path::is_safe_relative_asset_path, color::Hsi, registry::DefinitionMap};

/// One reusable model can be referenced by any number of creature definitions.
/// Model paths are relative to `assets/`, never absolute filesystem paths.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatureDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub model: String,
    #[serde(default)]
    pub textures: std::collections::HashMap<String, String>,
    pub collider: CreatureCollider,
    #[serde(default = "default_creature_health")]
    pub health: f32,
    #[serde(default = "default_creature_max_per_type")]
    pub max_per_type: usize,
    #[serde(default)]
    pub material_tints: std::collections::HashMap<String, Hsi>,
    #[serde(default)]
    pub unlit_materials: std::collections::HashSet<String>,
    #[serde(default)]
    pub particle_effects: std::collections::HashMap<String, CreatureParticleEffect>,
    #[serde(default)]
    pub animations: std::collections::HashMap<String, String>,
    #[serde(default = "default_jump_speed")]
    pub jump_speed: f32,
    #[serde(default)]
    pub move_speed: f32,
    #[serde(default = "default_jump_interval")]
    pub jump_interval: f32,
    #[serde(default)]
    pub anticipation_seconds: f32,
    #[serde(default)]
    pub landing_seconds: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatureParticleEffect {
    pub count: usize,
    pub color: [f32; 4],
    pub size: f32,
    pub lifetime: f32,
    #[serde(default)]
    pub interval: f32,
    #[serde(default)]
    pub spawn_radius: f32,
    #[serde(default)]
    pub y_offset: f32,
    #[serde(default)]
    pub horizontal_speed: f32,
    #[serde(default)]
    pub vertical_speed: f32,
    #[serde(default)]
    pub vertical_jitter: f32,
    #[serde(default)]
    pub gravity: f32,
    #[serde(default = "default_particle_end_scale")]
    pub end_scale: f32,
}

impl CreatureParticleEffect {
    fn validate(&self, creature_id: &str, state: &str) {
        assert!(
            (1..=64).contains(&self.count),
            "creature {creature_id} particle effect {state} count must be between 1 and 64"
        );
        assert!(
            self.color
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
            "creature {creature_id} particle effect {state} has an invalid RGBA color"
        );
        assert!(
            self.size.is_finite() && self.size > 0.0,
            "creature {creature_id} particle effect {state} size must be positive and finite"
        );
        assert!(
            self.lifetime.is_finite() && self.lifetime > 0.0,
            "creature {creature_id} particle effect {state} lifetime must be positive and finite"
        );
        assert!(
            self.interval.is_finite() && self.interval >= 0.0,
            "creature {creature_id} particle effect {state} interval must be finite and non-negative"
        );
        assert!(
            self.spawn_radius.is_finite() && self.spawn_radius >= 0.0,
            "creature {creature_id} particle effect {state} spawnRadius must be finite and non-negative"
        );
        assert!(
            self.y_offset.is_finite()
                && self.horizontal_speed.is_finite()
                && self.horizontal_speed >= 0.0
                && self.vertical_speed.is_finite()
                && self.vertical_jitter.is_finite()
                && self.vertical_jitter >= 0.0
                && self.gravity.is_finite(),
            "creature {creature_id} particle effect {state} has invalid motion values"
        );
        assert!(
            self.end_scale.is_finite() && self.end_scale >= 0.0,
            "creature {creature_id} particle effect {state} endScale must be finite and non-negative"
        );
    }
}

fn default_particle_end_scale() -> f32 {
    0.15
}

fn default_creature_health() -> f32 {
    10.0
}

fn default_creature_max_per_type() -> usize {
    4
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
        assert!(
            self.size
                .iter()
                .all(|value| value.is_finite() && *value > 0.0),
            "creature {id} collider size must be positive and finite"
        );
        assert!(
            self.center_offset.iter().all(|value| value.is_finite()),
            "creature {id} collider offset must be finite"
        );
    }
}

#[derive(Resource, Default)]
pub struct CreatureRegistry {
    definitions: DefinitionMap<CreatureDefinition>,
}

impl CreatureRegistry {
    pub fn insert(&mut self, definition: CreatureDefinition) {
        assert!(
            !definition.id.trim().is_empty(),
            "creature id cannot be empty"
        );
        definition
            .name
            .validate(&format!("creature {} name", definition.id));
        assert!(
            valid_creature_model_path(&definition.model),
            "creature {} model must be a safe relative .glb/.gltf path under assets/models/creatures/: {}",
            definition.id,
            definition.model
        );
        for (material, texture) in &definition.textures {
            assert!(
                !material.trim().is_empty() && valid_creature_texture_path(texture),
                "creature {} material {material:?} has an invalid texture path: {texture}",
                definition.id
            );
        }
        definition.collider.validate(&definition.id);
        assert!(
            definition.max_per_type > 0,
            "creature {} maxPerType must be positive",
            definition.id
        );
        assert!(
            definition.health.is_finite() && definition.health > 0.0,
            "creature {} health must be positive and finite",
            definition.id
        );
        assert!(
            definition.jump_speed.is_finite() && definition.jump_speed >= 0.0,
            "creature {} has invalid jumpSpeed",
            definition.id
        );
        assert!(
            definition.move_speed.is_finite() && definition.move_speed >= 0.0,
            "creature {} has invalid moveSpeed",
            definition.id
        );
        assert!(
            definition.jump_interval.is_finite() && definition.jump_interval > 0.0,
            "creature {} has invalid jumpInterval",
            definition.id
        );
        assert!(
            definition.anticipation_seconds.is_finite() && definition.anticipation_seconds >= 0.0,
            "creature {} has invalid anticipationSeconds",
            definition.id
        );
        assert!(
            definition.landing_seconds.is_finite() && definition.landing_seconds >= 0.0,
            "creature {} has invalid landingSeconds",
            definition.id
        );
        for (material, tint) in &definition.material_tints {
            assert!(
                !material.is_empty() && tint.is_valid(),
                "creature {} has an invalid material tint for {material}",
                definition.id
            );
        }
        for material in &definition.unlit_materials {
            assert!(
                !material.trim().is_empty(),
                "creature {} has an empty unlit material name",
                definition.id
            );
        }
        for (state, effect) in &definition.particle_effects {
            assert!(
                !state.trim().is_empty(),
                "creature {} has an empty particle effect state",
                definition.id
            );
            effect.validate(&definition.id, state);
        }
        for (state, clip) in &definition.animations {
            assert!(
                !state.is_empty() && !clip.is_empty(),
                "creature {} has an empty animation state/clip",
                definition.id
            );
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
    if !is_safe_relative_asset_path(path) {
        return false;
    }
    let candidate = Path::new(path);
    let mut parts = path.split('/');
    if parts.next() != Some("models") || parts.next() != Some("creatures") {
        return false;
    }
    matches!(
        candidate.extension().and_then(|ext| ext.to_str()),
        Some("glb" | "gltf")
    )
}

/// Creature materials may only refer to PNGs beneath the moddable creature texture root.
fn valid_creature_texture_path(path: &str) -> bool {
    if !is_safe_relative_asset_path(path) {
        return false;
    }
    let candidate = Path::new(path);
    let mut parts = path.split('/');
    if parts.next() != Some("textures") || parts.next() != Some("creatures") {
        return false;
    }
    matches!(
        candidate.extension().and_then(|ext| ext.to_str()),
        Some("png")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_paths_remain_inside_moddable_creatures_directory() {
        assert!(valid_creature_model_path(
            "models/creatures/slime/slime.glb"
        ));
        assert!(!valid_creature_model_path("models/creatures/../secret.glb"));
        assert!(!valid_creature_model_path("/models/creatures/slime.glb"));
        assert!(!valid_creature_model_path("models/blocks/stone.glb"));
        assert!(!valid_creature_model_path("models/creatures\\secret.glb"));
    }

    #[test]
    fn texture_paths_remain_inside_moddable_creatures_directory() {
        assert!(valid_creature_texture_path(
            "textures/creatures/slime.png"
        ));
        assert!(!valid_creature_texture_path("textures/creatures/../secret.png"));
        assert!(!valid_creature_texture_path(
            "/textures/creatures/slime.png"
        ));
        assert!(!valid_creature_texture_path("textures/blocks/stone.png"));
        assert!(!valid_creature_texture_path("textures/creatures/slime.jpg"));
        assert!(!valid_creature_texture_path(
            "textures/creatures\\secret.png"
        ));
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
