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

const MAX_OBJECT_VISUAL_SIZE: f32 = 4.0;
const MAX_OBJECT_VISUAL_OFFSET: f32 = 2.0;
const MAX_CROSSED_SPRITE_PLANES: u8 = 8;
const MAX_CUBOID_SET_PARTS: usize = 64;

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

fn default_crossed_sprite_width() -> f32 {
    0.75
}

fn default_crossed_sprite_planes() -> u8 {
    2
}

fn default_alpha_cutoff() -> f32 {
    0.5
}

fn default_uv_rect() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectCuboidPartDefinition {
    pub(crate) from: [f32; 3],
    pub(crate) to: [f32; 3],
    #[serde(default = "default_uv_rect")]
    pub(crate) uv: [f32; 4],
}

impl ObjectCuboidPartDefinition {
    fn validate(&self, object_id: &str, index: usize) {
        assert!(
            self.from
                .iter()
                .chain(self.to.iter())
                .all(|value| value.is_finite() && value.abs() <= MAX_OBJECT_VISUAL_SIZE),
            "object {object_id} cuboidSet parts[{index}] coordinates must be finite and within +/-{MAX_OBJECT_VISUAL_SIZE}"
        );
        assert!(
            self.from
                .iter()
                .zip(self.to.iter())
                .all(|(from, to)| from < to),
            "object {object_id} cuboidSet parts[{index}] to must be greater than from on every axis"
        );
        assert!(
            self.uv
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
            "object {object_id} cuboidSet parts[{index}] uv values must be between 0 and 1"
        );
        assert!(
            self.uv[0] < self.uv[2] && self.uv[1] < self.uv[3],
            "object {object_id} cuboidSet parts[{index}] uv must have positive width and height"
        );
    }
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
        #[serde(default = "default_extruded_sprite_size")]
        size: [f32; 2],
        #[serde(default = "default_alpha_cutoff", rename = "alphaCutoff")]
        alpha_cutoff: f32,
    },
    CrossedSprite {
        texture: String,
        #[serde(default, rename = "baseOffset")]
        base_offset: f32,
        #[serde(default = "default_crossed_sprite_width")]
        width: f32,
        height: f32,
        #[serde(default = "default_crossed_sprite_planes")]
        planes: u8,
        #[serde(default = "default_alpha_cutoff", rename = "alphaCutoff")]
        alpha_cutoff: f32,
    },
    CuboidSet {
        texture: String,
        parts: Vec<ObjectCuboidPartDefinition>,
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
                size,
                alpha_cutoff,
            } => {
                normalize_texture(texture, object_id, "extrudedSprite");
                validate_base_offset(*base_offset, object_id, "extrudedSprite");
                validate_positive_size(*height, object_id, "extrudedSprite height");
                assert!(
                    size.iter().all(|value| {
                        value.is_finite() && *value > 0.0 && *value <= MAX_OBJECT_VISUAL_SIZE
                    }),
                    "object {object_id} extrudedSprite size must be positive, finite and <= {MAX_OBJECT_VISUAL_SIZE}"
                );
                validate_alpha_cutoff(*alpha_cutoff, object_id, "extrudedSprite");
            }
            Self::CrossedSprite {
                texture,
                base_offset,
                width,
                height,
                planes,
                alpha_cutoff,
            } => {
                normalize_texture(texture, object_id, "crossedSprite");
                validate_base_offset(*base_offset, object_id, "crossedSprite");
                validate_positive_size(*width, object_id, "crossedSprite width");
                validate_positive_size(*height, object_id, "crossedSprite height");
                assert!(
                    (2..=MAX_CROSSED_SPRITE_PLANES).contains(planes),
                    "object {object_id} crossedSprite planes must be between 2 and {MAX_CROSSED_SPRITE_PLANES}"
                );
                validate_alpha_cutoff(*alpha_cutoff, object_id, "crossedSprite");
            }
            Self::CuboidSet {
                texture,
                parts,
                alpha_cutoff,
            } => {
                normalize_texture(texture, object_id, "cuboidSet");
                assert!(
                    !parts.is_empty() && parts.len() <= MAX_CUBOID_SET_PARTS,
                    "object {object_id} cuboidSet must contain between 1 and {MAX_CUBOID_SET_PARTS} parts"
                );
                for (index, part) in parts.iter().enumerate() {
                    part.validate(object_id, index);
                }
                validate_alpha_cutoff(*alpha_cutoff, object_id, "cuboidSet");
            }
        }
    }
}

fn normalize_texture(texture: &mut String, object_id: &str, visual_type: &str) {
    *texture = texture.trim().to_owned();
    assert!(
        is_safe_relative_asset_path(texture),
        "object {object_id} {visual_type} texture must be a safe relative asset path: {texture}"
    );
}

fn validate_base_offset(base_offset: f32, object_id: &str, visual_type: &str) {
    assert!(
        base_offset.is_finite() && (0.0..=MAX_OBJECT_VISUAL_OFFSET).contains(&base_offset),
        "object {object_id} {visual_type} baseOffset must be finite and between 0 and {MAX_OBJECT_VISUAL_OFFSET}"
    );
}

fn validate_positive_size(value: f32, object_id: &str, field: &str) {
    assert!(
        value.is_finite() && value > 0.0 && value <= MAX_OBJECT_VISUAL_SIZE,
        "object {object_id} {field} must be positive, finite and <= {MAX_OBJECT_VISUAL_SIZE}"
    );
}

fn validate_alpha_cutoff(alpha_cutoff: f32, object_id: &str, visual_type: &str) {
    assert!(
        alpha_cutoff.is_finite() && (0.0..=1.0).contains(&alpha_cutoff),
        "object {object_id} {visual_type} alphaCutoff must be between 0 and 1"
    );
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
                "height": 0.012,
                "size": [0.42, 0.42],
                "alphaCutoff": 0.5
            }"#,
        )
        .expect("extruded sprite visual should accept the public camelCase schema");

        let ObjectVisualDefinition::ExtrudedSprite {
            texture,
            base_offset,
            height,
            size,
            alpha_cutoff,
        } = visual
        else {
            panic!("expected extrudedSprite visual");
        };

        assert_eq!(texture, "textures/items/pebble.png");
        assert_eq!(base_offset, 0.0125);
        assert_eq!(height, 0.012);
        assert_eq!(size, [0.42, 0.42]);
        assert_eq!(alpha_cutoff, 0.5);
    }

    #[test]
    fn crossed_sprite_visual_deserializes() {
        let visual: ObjectVisualDefinition = serde_json::from_str(
            r#"{
                "type": "crossedSprite",
                "texture": "textures/objects/grass.png",
                "width": 0.72,
                "height": 0.62,
                "planes": 3
            }"#,
        )
        .expect("crossed sprite visual should deserialize");

        let ObjectVisualDefinition::CrossedSprite {
            texture,
            width,
            height,
            planes,
            ..
        } = visual
        else {
            panic!("expected crossedSprite visual");
        };
        assert_eq!(texture, "textures/objects/grass.png");
        assert_eq!(width, 0.72);
        assert_eq!(height, 0.62);
        assert_eq!(planes, 3);
    }

    #[test]
    fn cuboid_set_visual_deserializes() {
        let visual: ObjectVisualDefinition = serde_json::from_str(
            r#"{
                "type": "cuboidSet",
                "texture": "textures/objects/torch.png",
                "parts": [
                    {"from": [-0.08, 0.0, -0.08], "to": [0.08, 0.65, 0.08]}
                ]
            }"#,
        )
        .expect("cuboid set visual should deserialize");

        let ObjectVisualDefinition::CuboidSet { texture, parts, .. } = visual else {
            panic!("expected cuboidSet visual");
        };
        assert_eq!(texture, "textures/objects/torch.png");
        assert_eq!(parts.len(), 1);
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
