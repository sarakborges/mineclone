use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{block_id::intern_block_id, block_orientation::BlockOrientation};

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

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTextureRotations {
    #[serde(default)]
    pub top: bool,
    #[serde(default)]
    pub bottom: bool,
    #[serde(default)]
    pub left: bool,
    #[serde(default)]
    pub right: bool,
    #[serde(default)]
    pub front: bool,
    #[serde(default)]
    pub back: bool,
}

impl BlockTextureRotations {
    pub fn any(self) -> bool {
        self.top || self.bottom || self.left || self.right || self.front || self.back
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockTint {
    #[default]
    None,
    Grass,
    Leaf,
    Foliage,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tint: BlockTint,
    #[serde(default)]
    pub textures: BlockTextures,
    #[serde(default)]
    pub rotate_texture: BlockTextureRotations,
    #[serde(default)]
    pub orientations: Vec<BlockOrientation>,
    #[serde(default)]
    pub alpha_cutoff: Option<f32>,
    #[serde(default)]
    pub alpha_blend: bool,
    #[serde(default)]
    pub light_emission: u8,
    #[serde(default = "default_light_dampening")]
    pub light_dampening: u8,
    #[serde(default = "default_casts_shadow")]
    pub casts_shadow: bool,
}

impl BlockDefinition {
    pub fn alpha_mode(&self, opacity: f32) -> AlphaMode {
        if opacity < 1.0 || self.alpha_blend {
            AlphaMode::Blend
        } else if let Some(cutoff) = self.alpha_cutoff {
            AlphaMode::Mask(cutoff)
        } else {
            AlphaMode::Opaque
        }
    }

    pub fn default_orientation(&self) -> BlockOrientation {
        self.orientations.first().copied().unwrap_or_default()
    }

    pub fn next_orientation(&self, current: BlockOrientation) -> BlockOrientation {
        let Some(&first) = self.orientations.first() else {
            return BlockOrientation::default();
        };

        let Some(index) = self
            .orientations
            .iter()
            .position(|orientation| *orientation == current)
        else {
            return first;
        };

        self.orientations[(index + 1) % self.orientations.len()]
    }
}

#[derive(Resource, Default)]
pub struct BlockRegistry {
    definitions: HashMap<String, BlockDefinition>,
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
        if let Some(alpha_cutoff) = definition.alpha_cutoff {
            assert!(
                (0.0..=1.0).contains(&alpha_cutoff),
                "block {} alphaCutoff must be between 0 and 1",
                definition.id
            );
        }
        assert!(
            !definition.alpha_blend || definition.alpha_cutoff.is_none(),
            "block {} cannot use alphaBlend and alphaCutoff together",
            definition.id
        );
        for (index, orientation) in definition.orientations.iter().enumerate() {
            assert!(
                !definition.orientations[..index].contains(orientation),
                "block {} orientations cannot contain duplicates",
                definition.id
            );
        }

        intern_block_id(&definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.definitions.get(id)
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
