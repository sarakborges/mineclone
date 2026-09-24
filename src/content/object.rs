use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    asset_path::is_safe_relative_asset_path,
    block::BlockTint,
    inventory_category::InventoryCategoryRegistry,
    object_id::intern_object_id,
    registry::DefinitionMap,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
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
    #[expect(
        dead_code,
        reason = "object placement faces are part of the authored schema before runtime placement is wired"
    )]
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
    pub model: String,
    pub icon: String,
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "object rendering fields are authored before runtime object rendering is wired"
    )]
    pub tint: BlockTint,
    #[serde(default = "default_placement_faces")]
    pub placement_faces: Vec<ObjectPlacementFace>,
    #[serde(default)]
    pub target: ObjectTargetDefinition,
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "object rendering fields are authored before runtime object rendering is wired"
    )]
    pub unlit: bool,
    #[serde(default = "default_true")]
    #[expect(
        dead_code,
        reason = "object runtime fields are authored before their consuming pipeline is wired"
    )]
    pub casts_shadow: bool,
    #[serde(default = "default_true")]
    #[expect(
        dead_code,
        reason = "object runtime fields are authored before their consuming pipeline is wired"
    )]
    pub receives_shadow: bool,
    #[serde(default = "default_true")]
    #[expect(
        dead_code,
        reason = "object runtime fields are authored before their consuming pipeline is wired"
    )]
    pub drop_self: bool,
}

impl ObjectDefinition {
    #[expect(
        dead_code,
        reason = "object placement rules are authored before runtime placement is wired"
    )]
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
        definition.model = definition.model.trim().to_owned();
        definition.icon = definition.icon.trim().to_owned();

        assert!(!definition.id.is_empty(), "object id cannot be empty");
        assert!(
            !definition.category.is_empty(),
            "object {} category cannot be empty",
            definition.id
        );
        assert!(
            is_safe_relative_asset_path(&definition.model),
            "object {} model must be a safe relative asset path: {}",
            definition.id,
            definition.model
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
