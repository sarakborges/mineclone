use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    asset_path::is_safe_relative_asset_path, block::BlockTint, layer_id::intern_layer_id,
    registry::DefinitionMap,
};

const DEFAULT_LAYER_OFFSET: f32 = 1.0 / 1024.0;
const MAX_LAYER_OFFSET: f32 = 1.0 / 8.0;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LayerFace {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

impl LayerFace {
    pub const ALL: [Self; 6] = [
        Self::Right,
        Self::Left,
        Self::Top,
        Self::Bottom,
        Self::Front,
        Self::Back,
    ];

    pub(crate) fn from_normal(normal: IVec3) -> Option<Self> {
        match normal {
            IVec3::X => Some(Self::Right),
            IVec3::NEG_X => Some(Self::Left),
            IVec3::Y => Some(Self::Top),
            IVec3::NEG_Y => Some(Self::Bottom),
            IVec3::Z => Some(Self::Front),
            IVec3::NEG_Z => Some(Self::Back),
            _ => None,
        }
    }

    pub(crate) fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Right),
            1 => Some(Self::Left),
            2 => Some(Self::Top),
            3 => Some(Self::Bottom),
            4 => Some(Self::Front),
            5 => Some(Self::Back),
            _ => None,
        }
    }

    pub(crate) fn index(self) -> u8 {
        match self {
            Self::Right => 0,
            Self::Left => 1,
            Self::Top => 2,
            Self::Bottom => 3,
            Self::Front => 4,
            Self::Back => 5,
        }
    }
}

fn default_faces() -> Vec<LayerFace> {
    LayerFace::ALL.to_vec()
}

fn default_offset() -> f32 {
    DEFAULT_LAYER_OFFSET
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub category: String,
    pub texture: String,
    #[serde(default)]
    pub tint: BlockTint,
    #[serde(default = "default_faces")]
    pub faces: Vec<LayerFace>,
    #[serde(default = "default_offset")]
    pub offset: f32,
    #[serde(default)]
    pub alpha_cutoff: Option<f32>,
    #[serde(default)]
    pub alpha_blend: bool,
    #[serde(default)]
    pub casts_shadow: bool,
}

impl LayerDefinition {
    pub(crate) fn supports_face(&self, face: LayerFace) -> bool {
        self.faces.contains(&face)
    }
}

#[derive(Resource, Default, Clone)]
pub struct LayerRegistry {
    definitions: DefinitionMap<LayerDefinition>,
}

impl LayerRegistry {
    pub fn insert(&mut self, mut definition: LayerDefinition) {
        assert!(!definition.id.trim().is_empty(), "layer id cannot be empty");
        definition
            .name
            .validate(&format!("layer {} name", definition.id));
        definition.category = definition.category.trim().to_owned();
        assert!(
            !definition.category.is_empty(),
            "layer {} category cannot be empty",
            definition.id
        );
        assert!(
            is_safe_relative_asset_path(&definition.texture),
            "layer {} texture must be a safe relative asset path: {}",
            definition.id,
            definition.texture
        );
        assert!(
            definition.offset.is_finite()
                && (0.0..=MAX_LAYER_OFFSET).contains(&definition.offset),
            "layer {} offset must be finite and between 0 and {MAX_LAYER_OFFSET}",
            definition.id
        );
        assert!(
            !definition.faces.is_empty(),
            "layer {} must support at least one face",
            definition.id
        );
        for (index, face) in definition.faces.iter().enumerate() {
            assert!(
                !definition.faces[..index].contains(face),
                "layer {} faces cannot contain duplicates",
                definition.id
            );
        }
        if let Some(alpha_cutoff) = definition.alpha_cutoff {
            assert!(
                (0.0..=1.0).contains(&alpha_cutoff),
                "layer {} alphaCutoff must be between 0 and 1",
                definition.id
            );
        }
        assert!(
            !definition.alpha_blend || definition.alpha_cutoff.is_none(),
            "layer {} cannot use alphaBlend and alphaCutoff together",
            definition.id
        );

        intern_layer_id(&definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&LayerDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &LayerDefinition> {
        self.definitions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_preserves_allowed_faces() {
        let mut registry = LayerRegistry::default();
        registry.insert(LayerDefinition {
            id: "asteria:test_layer".to_owned(),
            name: serde_json::from_str(
                r#"{"english":"Test Layer","portuguese_brazil":"Test Layer","spanish":"Test Layer"}"#,
            )
            .unwrap(),
            category: "natural_blocks".to_owned(),
            texture: "textures/test.png".to_owned(),
            tint: BlockTint::None,
            faces: vec![LayerFace::Top, LayerFace::Front],
            offset: DEFAULT_LAYER_OFFSET,
            alpha_cutoff: Some(0.5),
            alpha_blend: false,
            casts_shadow: false,
        });

        let layer = registry.get("asteria:test_layer").unwrap();
        assert!(layer.supports_face(LayerFace::Top));
        assert!(layer.supports_face(LayerFace::Front));
        assert!(!layer.supports_face(LayerFace::Bottom));
    }
}
