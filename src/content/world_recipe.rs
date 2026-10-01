use bevy::prelude::*;
use serde::Deserialize;

use super::{
    block::BlockRegistry,
    object::{ObjectPlacementFace, ObjectRegistry},
    registry::DefinitionMap,
};

fn default_object_placement_face() -> ObjectPlacementFace {
    ObjectPlacementFace::Top
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorldRecipeIngredientDefinition {
    pub(crate) item: String,
    pub(crate) quantity: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum WorldRecipeResultDefinition {
    ReplaceBlockWithObject {
        object: String,
        #[serde(default = "default_object_placement_face")]
        placement_face: ObjectPlacementFace,
    },
}

impl WorldRecipeResultDefinition {
    pub(crate) fn object_placement(&self) -> (&str, ObjectPlacementFace) {
        match self {
            Self::ReplaceBlockWithObject {
                object,
                placement_face,
            } => (object, *placement_face),
        }
    }

    fn normalize(&mut self) {
        match self {
            Self::ReplaceBlockWithObject { object, .. } => {
                *object = object.trim().to_owned();
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorldRecipeDefinition {
    pub(crate) id: String,
    pub(crate) target_block: String,
    pub(crate) held_item: String,
    pub(crate) ingredients: Vec<WorldRecipeIngredientDefinition>,
    pub(crate) result: WorldRecipeResultDefinition,
}

impl WorldRecipeDefinition {
    fn normalize_and_validate(&mut self) {
        self.id = self.id.trim().to_owned();
        self.target_block = self.target_block.trim().to_owned();
        self.held_item = self.held_item.trim().to_owned();
        self.result.normalize();

        assert!(!self.id.is_empty(), "world recipe id cannot be empty");
        assert!(
            !self.target_block.is_empty(),
            "world recipe {} targetBlock cannot be empty",
            self.id
        );
        assert!(
            !self.held_item.is_empty(),
            "world recipe {} heldItem cannot be empty",
            self.id
        );
        assert!(
            !self.ingredients.is_empty(),
            "world recipe {} must contain at least one ingredient",
            self.id
        );

        for index in 0..self.ingredients.len() {
            let (previous, current_and_after) = self.ingredients.split_at_mut(index);
            let ingredient = &mut current_and_after[0];
            ingredient.item = ingredient.item.trim().to_owned();
            assert!(
                !ingredient.item.is_empty(),
                "world recipe {} ingredients[{index}].item cannot be empty",
                self.id
            );
            assert!(
                ingredient.quantity > 0,
                "world recipe {} ingredients[{index}].quantity must be greater than zero",
                self.id
            );
            assert!(
                !previous
                    .iter()
                    .any(|candidate| candidate.item == ingredient.item),
                "world recipe {} contains duplicate ingredient {}",
                self.id,
                ingredient.item
            );
        }

        let (object, _) = self.result.object_placement();
        assert!(
            !object.is_empty(),
            "world recipe {} result object cannot be empty",
            self.id
        );
    }

    pub(crate) fn validate_references(
        &self,
        blocks: &BlockRegistry,
        objects: &ObjectRegistry,
        item_exists: impl Fn(&str) -> bool,
    ) {
        assert!(
            blocks.get(&self.target_block).is_some(),
            "world recipe {} references missing target block {}",
            self.id,
            self.target_block
        );
        assert!(
            item_exists(&self.held_item),
            "world recipe {} references missing held item {}",
            self.id,
            self.held_item
        );
        for ingredient in &self.ingredients {
            assert!(
                item_exists(&ingredient.item),
                "world recipe {} references missing ingredient {}",
                self.id,
                ingredient.item
            );
        }

        let (object_id, placement_face) = self.result.object_placement();
        let object = objects.get(object_id).unwrap_or_else(|| {
            panic!(
                "world recipe {} references missing result object {}",
                self.id, object_id
            )
        });
        assert!(
            object.supports_placement_face(placement_face),
            "world recipe {} result object {} does not support placement face {:?}",
            self.id,
            object_id,
            placement_face
        );
    }
}

#[derive(Resource, Default, Clone)]
pub(crate) struct WorldRecipeRegistry {
    definitions: DefinitionMap<WorldRecipeDefinition>,
}

impl WorldRecipeRegistry {
    pub(crate) fn insert(&mut self, mut definition: WorldRecipeDefinition) {
        definition.normalize_and_validate();
        assert!(
            !self.definitions.values().any(|existing| {
                existing.target_block == definition.target_block
                    && existing.held_item == definition.held_item
            }),
            "world recipe {} duplicates trigger targetBlock={} heldItem={}",
            definition.id,
            definition.target_block,
            definition.held_item
        );
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub(crate) fn matching(
        &self,
        target_block: &str,
        held_item: &str,
    ) -> Option<&WorldRecipeDefinition> {
        self.definitions.values().find(|definition| {
            definition.target_block == target_block && definition.held_item == held_item
        })
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &WorldRecipeDefinition> {
        self.definitions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_recipe_deserializes_replace_block_with_object() {
        let recipe: WorldRecipeDefinition = serde_json::from_str(
            r#"{
                "id": "asteria:test_recipe",
                "targetBlock": "asteria:stone",
                "heldItem": "asteria:pebble",
                "ingredients": [{"item": "asteria:pebble", "quantity": 5}],
                "result": {
                    "type": "replaceBlockWithObject",
                    "object": "asteria:rustic_workbench"
                }
            }"#,
        )
        .expect("world recipe should deserialize");

        assert_eq!(recipe.target_block, "asteria:stone");
        assert_eq!(recipe.held_item, "asteria:pebble");
        assert_eq!(recipe.ingredients[0].quantity, 5);
        assert_eq!(
            recipe.result.object_placement(),
            ("asteria:rustic_workbench", ObjectPlacementFace::Top)
        );
    }

    #[test]
    #[should_panic(expected = "duplicates trigger")]
    fn duplicate_world_recipe_triggers_are_rejected() {
        let json = r#"{
            "id": "asteria:first",
            "targetBlock": "asteria:stone",
            "heldItem": "asteria:pebble",
            "ingredients": [{"item": "asteria:pebble", "quantity": 1}],
            "result": {
                "type": "replaceBlockWithObject",
                "object": "asteria:rustic_workbench"
            }
        }"#;
        let mut registry = WorldRecipeRegistry::default();
        registry.insert(serde_json::from_str(json).unwrap());
        let mut duplicate: WorldRecipeDefinition = serde_json::from_str(json).unwrap();
        duplicate.id = "asteria:second".to_owned();
        registry.insert(duplicate);
    }
}
