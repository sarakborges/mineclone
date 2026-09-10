use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::block_id::intern_block_id;

const MAX_LIGHT_DAMPENING: u8 = 15;

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTextures {
    pub top: String,
    pub bottom: String,
    pub left: String,
    pub right: String,
    pub front: String,
    pub back: String,
}

impl BlockTextures {
    pub fn is_empty(&self) -> bool {
        self.top.is_empty()
            && self.bottom.is_empty()
            && self.left.is_empty()
            && self.right.is_empty()
            && self.front.is_empty()
            && self.back.is_empty()
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
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
    static_ids: HashMap<String, &'static str>,
}

impl BlockRegistry {
    pub fn insert(&mut self, definition: BlockDefinition) {
        assert!(
            definition.light_emission <= MAX_LIGHT_DAMPENING,
            "block {} light emission must be between 0 and 15",
            definition.id
        );
        assert!(
            definition.light_dampening <= MAX_LIGHT_DAMPENING,
            "block {} light dampening must be between 0 and 15",
            definition.id
        );

        let static_id = intern_block_id(&definition.id);
        self.static_ids.insert(definition.id.clone(), static_id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.definitions.get(id)
    }

    pub fn static_id(&self, id: &str) -> Option<&'static str> {
        self.static_ids.get(id).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockDefinition> {
        self.definitions.values()
    }
}

fn default_light_dampening() -> u8 {
    MAX_LIGHT_DAMPENING
}

fn default_casts_shadow() -> bool {
    true
}
