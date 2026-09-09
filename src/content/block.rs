use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

const MAX_LIGHT_DAMPENING: u8 = 15;

#[derive(Clone, Deserialize)]
pub struct BlockTextures {
    pub top: String,
    pub bottom: String,
    pub left: String,
    pub right: String,
    pub front: String,
    pub back: String,
}

#[derive(Clone, Deserialize)]
pub struct BlockDefinition {
    pub id: String,
    pub name: String,
    pub textures: BlockTextures,
    pub rotate_texture: bool,
    #[serde(default)]
    pub light_emission: u8,
    #[serde(default = "default_light_dampening")]
    pub light_dampening: u8,
    #[serde(default = "default_casts_shadow")]
    pub casts_shadow: bool,
}

#[derive(Resource, Default)]
pub struct BlockRegistry {
    definitions: HashMap<String, BlockDefinition>,
}

impl BlockRegistry {
    pub fn insert(&mut self, definition: BlockDefinition) {
        assert!(
            definition.light_emission <= 15,
            "block {} light emission must be between 0 and 15",
            definition.id
        );
        assert!(
            definition.light_dampening <= MAX_LIGHT_DAMPENING,
            "block {} light dampening must be between 0 and 15",
            definition.id
        );

        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.definitions.get(id)
    }
}

fn default_light_dampening() -> u8 {
    MAX_LIGHT_DAMPENING
}

fn default_casts_shadow() -> bool {
    true
}
