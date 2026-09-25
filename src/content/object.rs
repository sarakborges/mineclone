use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    asset_path::is_safe_relative_asset_path,
    block::BlockTint,
    inventory_category::InventoryCategoryRegistry,
    loot::LootTableDefinition,
    object_id::intern_object_id,
    registry::DefinitionMap,
};

const MAX_EXTRUDED_SPRITE_SIZE: f32 = 4.0;
const MAX_EXTRUDED_SPRITE_OFFSET: f32 = 2.0;
const MAX_EXTRUDED_SPRITE_REPEATS: u32 = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ObjectPlacementFace {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

impl ObjectPlacementFace {
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

    #[allow(
        dead_code,
        reason = "voxel-backed world-object runtime migration has not consumed face normals yet"
    )]
    pub(crate) fn normal(self) -> IVec3 {
        match self {
            Self::Right => IVec3::X,
            Self::Left => IVec3::NEG_X,
            Self::Top => IVec3::Y,
            Self::Bottom => IVec3::NEG_Y,
            Self::Front => IVec3::Z,
            Self::Back => IVec3::NEG_Z,
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
}

fn default_placement_faces() -> Vec<ObjectPlacementFace> {
    vec![ObjectPlacementFace::Top]
}

fn default_target_size() -> [f32; 3] {
    [0.75, 0.75, 0.75]
}

fn default_target_center_offset() -> [f32; 3] {
    [0.0, 0.375, 0.0]
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ObjectInteraction {
    #[default]
    Break,
    Pickup,
}

fn default_position_jitter() -> [f32; 2] {
    [0.0, 0.0]
}


fn default_extruded_sprite_size() -> [f32; 2] {
    [0.75, 0.75]
}

fn default_alpha_cutoff() -> f32 {
    0.5
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ObjectVisualDefinition {
    Model {
        path: String,
    },
    ExtrudedSprite {
        texture: String,
        #[serde(default, rename = "baseOffset")]
        base_offset: f32,
        height: f32,
        #[serde(rename = "repeatHeight")]
        repeat_height: f32,
        #[serde(default = "default_extruded_sprite_size")]
        size: [f32; 2],
        #[serde(default = "default_alpha_cutoff", rename = "alphaCutoff")]
        alpha_cutoff: f32,
    },
}

impl ObjectVisualDefinition {
    fn normalize_and_validate(&mut self, object_id: &str) {
        match self {
            Self::Model { path } => {
                *path = path.trim().to_owned();
                assert!(
                    is_safe_relative_asset_path(path),
                    "object {object_id} model path must be a safe relative asset path: {path}"
                );
            }
            Self::ExtrudedSprite {
                texture,
                base_offset,
                height,
                repeat_height,
                size,
                alpha_cutoff,
            } => {
                *texture = texture.trim().to_owned();
                assert!(
                    is_safe_relative_asset_path(texture),
                    "object {object_id} extruded sprite texture must be a safe relative asset path: {texture}"
                );
                assert!(
                    base_offset.is_finite()
                        && (0.0..=MAX_EXTRUDED_SPRITE_OFFSET).contains(base_offset),
                    "object {object_id} extrudedSprite baseOffset must be finite and between 0 and {MAX_EXTRUDED_SPRITE_OFFSET}"
                );
                assert!(
                    height.is_finite() && *height > 0.0 && *height <= MAX_EXTRUDED_SPRITE_SIZE,
                    "object {object_id} extrudedSprite height must be positive, finite and <= {MAX_EXTRUDED_SPRITE_SIZE}"
                );
                assert!(
                    repeat_height.is_finite()
                        && *repeat_height > 0.0
                        && *repeat_height <= MAX_EXTRUDED_SPRITE_SIZE,
                    "object {object_id} extrudedSprite repeatHeight must be positive, finite and <= {MAX_EXTRUDED_SPRITE_SIZE}"
                );
                assert!(
                    *height / *repeat_height <= MAX_EXTRUDED_SPRITE_REPEATS as f32,
                    "object {object_id} extrudedSprite cannot exceed {MAX_EXTRUDED_SPRITE_REPEATS} vertical repeats"
                );
                assert!(
                    size.iter().all(|value| {
                        value.is_finite() && *value > 0.0 && *value <= MAX_EXTRUDED_SPRITE_SIZE
                    }),
                    "object {object_id} extrudedSprite size must be positive, finite and <= {MAX_EXTRUDED_SPRITE_SIZE}"
                );
                assert!(
                    alpha_cutoff.is_finite() && (0.0..=1.0).contains(alpha_cutoff),
                    "object {object_id} extrudedSprite alphaCutoff must be between 0 and 1"
                );
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTargetDefinition {
    #[serde(default = "default_target_size")]
    pub size: [f32; 3],
    #[serde(default = "default_target_center_offset")]
    pub center_offset: [f32; 3],
}

impl Default for ObjectTargetDefinition {
    fn default() -> Self {
        Self {
            size: default_target_size(),
            center_offset: default_target_center_offset(),
        }
    }
}

impl ObjectTargetDefinition {
    fn validate(self, object_id: &str) {
        assert!(
            self.size.iter().all(|value| value.is_finite() && *value > 0.0),
            "object {object_id} target size must be positive and finite"
        );
        assert!(
            self.center_offset.iter().all(|value| value.is_finite()),
            "object {object_id} target center offset must be finite"
        );
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub category: String,
    pub visual: ObjectVisualDefinition,
    pub icon: String,
    #[serde(default)]
    pub tint: BlockTint,
    #[serde(default = "default_placement_faces")]
    pub placement_faces: Vec<ObjectPlacementFace>,
    #[serde(default)]
    pub target: ObjectTargetDefinition,
    #[serde(default)]
    pub interaction: ObjectInteraction,
    #[serde(default = "default_position_jitter")]
    pub position_jitter: [f32; 2],
    #[serde(default)]
    pub(crate) loot_table: LootTableDefinition,
    #[serde(default)]
    pub unlit: bool,
    #[serde(default = "default_true")]
    pub casts_shadow: bool,
    #[serde(default = "default_true")]
    pub receives_shadow: bool,
    #[serde(default = "default_true")]
    pub drop_self: bool,
}

impl ObjectDefinition {
    pub(crate) fn supports_placement_face(&self, face: ObjectPlacementFace) -> bool {
        self.placement_faces.contains(&face)
    }

    pub(crate) fn validate_references(&self, categories: &InventoryCategoryRegistry) {
        assert!(
            categories.get(&self.category).is_some(),
            "object {} references missing inventory category {}",
            self.id,
            self.category
        );
    }
}

#[derive(Resource, Default, Clone)]
pub struct ObjectRegistry {
    definitions: DefinitionMap<ObjectDefinition>,
}

impl ObjectRegistry {
    pub fn insert(&mut self, mut definition: ObjectDefinition) {
        definition.id = definition.id.trim().to_owned();
        definition.category = definition.category.trim().to_owned();
        definition.icon = definition.icon.trim().to_owned();
        definition
            .visual
            .normalize_and_validate(definition.id.as_str());

        assert!(!definition.id.is_empty(), "object id cannot be empty");
        assert!(
            !definition.category.is_empty(),
            "object {} category cannot be empty",
            definition.id
        );
        assert!(
            is_safe_relative_asset_path(&definition.icon),
            "object {} icon must be a safe relative asset path: {}",
            definition.id,
            definition.icon
        );
        assert!(
            !definition.placement_faces.is_empty(),
            "object {} must support at least one placement face",
            definition.id
        );
        for (index, face) in definition.placement_faces.iter().enumerate() {
            assert!(
                !definition.placement_faces[..index].contains(face),
                "object {} placementFaces cannot contain duplicates",
                definition.id
            );
        }
        assert!(
            definition.position_jitter.iter().all(|value| {
                value.is_finite() && (0.0..=0.45).contains(value)
            }),
            "object {} positionJitter values must be finite and between 0 and 0.45",
            definition.id
        );
        definition
            .loot_table
            .validate(&format!("object {}", definition.id));
        definition.target.validate(&definition.id);
        definition
            .name
            .validate(&format!("object {} name", definition.id));
        intern_object_id(&definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&ObjectDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ObjectDefinition> {
        self.definitions.values()
    }
}


#[cfg(test)]
mod tests {
    use super::ObjectVisualDefinition;

    #[test]
    fn extruded_sprite_visual_deserializes_camel_case_fields() {
        let visual: ObjectVisualDefinition = serde_json::from_str(
            r#"{
                "type": "extrudedSprite",
                "texture": "textures/items/pebble.png",
                "baseOffset": 0.0125,
                "height": 0.1,
                "repeatHeight": 0.025,
                "size": [0.42, 0.42],
                "alphaCutoff": 0.5
            }"#,
        )
        .expect("extruded sprite visual should accept the public camelCase schema");

        let ObjectVisualDefinition::ExtrudedSprite {
            texture,
            base_offset,
            height,
            repeat_height,
            size,
            alpha_cutoff,
        } = visual
        else {
            panic!("expected extrudedSprite visual");
        };

        assert_eq!(texture, "textures/items/pebble.png");
        assert_eq!(base_offset, 0.0125);
        assert_eq!(height, 0.1);
        assert_eq!(repeat_height, 0.025);
        assert_eq!(size, [0.42, 0.42]);
        assert_eq!(alpha_cutoff, 0.5);
    }

    #[test]
    fn removed_sprite_prism_visual_is_rejected() {
        let result = serde_json::from_str::<ObjectVisualDefinition>(
            r#"{
                "type": "spritePrism",
                "texture": "textures/items/pebble.png",
                "baseOffset": 0.0125,
                "height": 0.1,
                "tileHeight": 0.025
            }"#,
        );
        assert!(result.is_err(), "spritePrism must stay removed");
    }
}
