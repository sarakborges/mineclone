use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::color::Rgb;

pub type FluidId = u16;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FluidDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub color: Rgb,
    pub opacity: f32,
    pub roughness: f32,
    #[serde(default)]
    pub metallic: f32,
    #[serde(default)]
    pub light_dampening: u8,
    pub spread_speed: f32,
    pub max_spread: u16,
}

#[derive(Resource, Default)]
pub struct FluidRegistry {
    definitions: Vec<FluidDefinition>,
    ids: HashMap<String, FluidId>,
}

impl FluidRegistry {
    pub fn insert(&mut self, definition: FluidDefinition) {
        definition
            .name
            .validate(&format!("fluid {} name", definition.id));
        assert!(
            (0.0..=1.0).contains(&definition.opacity),
            "fluid {} opacity must be between 0 and 1",
            definition.id
        );
        assert!(
            (0.0..=1.0).contains(&definition.roughness),
            "fluid {} roughness must be between 0 and 1",
            definition.id
        );
        assert!(
            (0.0..=1.0).contains(&definition.metallic),
            "fluid {} metallic must be between 0 and 1",
            definition.id
        );
        assert!(
            definition.light_dampening <= 15,
            "fluid {} light dampening must be between 0 and 15",
            definition.id
        );
        assert!(
            definition.spread_speed.is_finite() && definition.spread_speed >= 0.0,
            "fluid {} spread speed must be finite and non-negative",
            definition.id
        );

        if let Some(&fluid_id) = self.ids.get(&definition.id) {
            self.definitions[fluid_id as usize] = definition;
            return;
        }

        let index = self.definitions.len();
        assert!(
            index <= u16::MAX as usize,
            "fluid registry cannot exceed {} definitions",
            u16::MAX
        );
        let fluid_id = index as FluidId;

        self.ids.insert(definition.id.clone(), fluid_id);
        self.definitions.push(definition);
    }

    pub fn id_of(&self, id: &str) -> Option<FluidId> {
        self.ids.get(id).copied()
    }

    pub fn get(&self, fluid_id: FluidId) -> Option<&FluidDefinition> {
        self.definitions.get(fluid_id as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = (FluidId, &FluidDefinition)> {
        self.definitions
            .iter()
            .enumerate()
            .map(|(index, definition)| (index as FluidId, definition))
    }
}
