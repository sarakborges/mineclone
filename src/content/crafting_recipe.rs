use bevy::prelude::*;
use serde::Deserialize;

use super::{object::ObjectRegistry, registry::DefinitionMap};

fn default_result_quantity() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CraftingRecipeIngredientDefinition {
    pub(crate) item: String,
    pub(crate) quantity: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CraftingRecipeResultDefinition {
    pub(crate) item: String,
    #[serde(default = "default_result_quantity")]
    pub(crate) quantity: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CraftingRecipeDefinition {
    pub(crate) id: String,
    pub(crate) environment: String,
    pub(crate) ingredients: Vec<CraftingRecipeIngredientDefinition>,
    pub(crate) result: CraftingRecipeResultDefinition,
}

impl CraftingRecipeDefinition {
    fn normalize_and_validate(&mut self) {
        self.id = self.id.trim().to_owned();
        self.environment = self.environment.trim().to_owned();
        self.result.item = self.result.item.trim().to_owned();

        assert!(!self.id.is_empty(), "crafting recipe id cannot be empty");
        assert!(
            !self.environment.is_empty(),
            "crafting recipe {} environment cannot be empty",
            self.id
        );
        assert!(
            !self.ingredients.is_empty(),
            "crafting recipe {} must contain at least one ingredient",
            self.id
        );
        assert!(
            !self.result.item.is_empty(),
            "crafting recipe {} result item cannot be empty",
            self.id
        );
        assert!(
            self.result.quantity > 0,
            "crafting recipe {} result quantity must be greater than zero",
            self.id
        );

        for index in 0..self.ingredients.len() {
            let (previous, current_and_after) = self.ingredients.split_at_mut(index);
            let ingredient = &mut current_and_after[0];
            ingredient.item = ingredient.item.trim().to_owned();

            assert!(
                !ingredient.item.is_empty(),
                "crafting recipe {} ingredients[{index}].item cannot be empty",
                self.id
            );
            assert!(
                ingredient.quantity > 0,
                "crafting recipe {} ingredients[{index}].quantity must be greater than zero",
                self.id
            );
            assert!(
                !previous
                    .iter()
                    .any(|candidate| candidate.item == ingredient.item),
                "crafting recipe {} contains duplicate ingredient {}",
                self.id,
                ingredient.item
            );
        }
    }

    pub(crate) fn validate_references(
        &self,
        _objects: &ObjectRegistry,
        item_exists: impl Fn(&str) -> bool,
    ) {
        for ingredient in &self.ingredients {
            assert!(
                item_exists(&ingredient.item),
                "crafting recipe {} references missing ingredient {}",
                self.id,
                ingredient.item
            );
        }
        assert!(
            item_exists(&self.result.item),
            "crafting recipe {} references missing result item {}",
            self.id,
            self.result.item
        );
    }
}

#[derive(Resource, Default, Clone)]
pub(crate) struct CraftingRecipeRegistry {
    definitions: DefinitionMap<CraftingRecipeDefinition>,
}

impl CraftingRecipeRegistry {
    pub(crate) fn insert(&mut self, mut definition: CraftingRecipeDefinition) {
        definition.normalize_and_validate();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub(crate) fn get(&self, id: &str) -> Option<&CraftingRecipeDefinition> {
        self.definitions.get(id)
    }

    pub(crate) fn for_environment<'a>(
        &'a self,
        environment: &'a str,
    ) -> impl Iterator<Item = &'a CraftingRecipeDefinition> + 'a {
        self.definitions
            .values()
            .filter(move |definition| definition.environment == environment)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &CraftingRecipeDefinition> {
        self.definitions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crafting_recipe_deserializes() {
        let recipe: CraftingRecipeDefinition = serde_json::from_str(
            r#"{
                "id": "asteria:test_hatchet",
                "environment": "inventory",
                "ingredients": [
                    {"item": "asteria:pebble", "quantity": 2},
                    {"item": "asteria:stick", "quantity": 3}
                ],
                "result": {"item": "asteria:hatchet_rustic"}
            }"#,
        )
        .expect("crafting recipe should deserialize");

        assert_eq!(recipe.environment, "inventory");
        assert_eq!(recipe.ingredients[0].quantity, 2);
        assert_eq!(recipe.result.item, "asteria:hatchet_rustic");
        assert_eq!(recipe.result.quantity, 1);
    }
}
