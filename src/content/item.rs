use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::{Language, LocalizedText, UiLocalization};

use super::{
    asset_path::is_safe_relative_asset_path, block::BlockRegistry,
    builtin_ids::BUCKET_FLUID_METADATA_KEY, fluid::FluidRegistry,
    inventory_category::InventoryCategoryRegistry, item_id::intern_item_id, layer::LayerRegistry,
    object::ObjectRegistry, registry::DefinitionMap, tool::ToolRegistry,
};

const BUCKET_TOOL_ID: &str = "asteria:bucket";

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemIconVariantDefinition {
    pub metadata_key: String,
    pub metadata_value: String,
    pub icon: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub category: String,
    pub icon: String,
    #[serde(default)]
    pub icon_variants: Vec<ItemIconVariantDefinition>,
}

impl ItemDefinition {
    pub(crate) fn icon_variant(&self, metadata_key: &str, metadata_value: &str) -> Option<&str> {
        self.icon_variants
            .iter()
            .find(|variant| {
                variant.metadata_key == metadata_key && variant.metadata_value == metadata_value
            })
            .map(|variant| variant.icon.as_str())
    }

    pub(crate) fn icon_for_metadata<'a, 'm>(
        &'a self,
        metadata: impl IntoIterator<Item = (&'m str, &'m str)>,
    ) -> &'a str {
        metadata
            .into_iter()
            .find_map(|(key, value)| self.icon_variant(key, value))
            .unwrap_or(&self.icon)
    }

    pub(crate) fn validate_references(&self, categories: &InventoryCategoryRegistry) {
        assert!(
            categories.get(&self.category).is_some(),
            "item {} references missing inventory category {}",
            self.id,
            self.category
        );
    }
}

#[derive(Resource, Default)]
pub struct ItemRegistry {
    definitions: DefinitionMap<ItemDefinition>,
}

impl ItemRegistry {
    pub fn insert(&mut self, mut definition: ItemDefinition) {
        definition.id = definition.id.trim().to_owned();
        definition.category = definition.category.trim().to_owned();
        definition.icon = definition.icon.trim().to_owned();

        assert!(!definition.id.is_empty(), "item id cannot be empty");
        assert!(
            !definition.category.is_empty(),
            "item {} category cannot be empty",
            definition.id
        );
        assert!(
            !definition.icon.is_empty(),
            "item {} icon cannot be empty",
            definition.id
        );
        assert!(
            is_safe_relative_asset_path(&definition.icon),
            "item {} icon must be a safe relative asset path: {}",
            definition.id,
            definition.icon
        );
        for index in 0..definition.icon_variants.len() {
            let variant = &mut definition.icon_variants[index];
            variant.metadata_key = variant.metadata_key.trim().to_owned();
            variant.metadata_value = variant.metadata_value.trim().to_owned();
            variant.icon = variant.icon.trim().to_owned();
            assert!(
                !variant.metadata_key.is_empty(),
                "item {} icon variant metadataKey cannot be empty",
                definition.id
            );
            assert!(
                !variant.metadata_value.is_empty(),
                "item {} icon variant metadataValue cannot be empty",
                definition.id
            );
            assert!(
                is_safe_relative_asset_path(&variant.icon),
                "item {} icon variant must be a safe relative asset path: {}",
                definition.id,
                variant.icon
            );
            assert!(
                !definition.icon_variants[..index].iter().any(|earlier| {
                    earlier.metadata_key == variant.metadata_key
                        && earlier.metadata_value == variant.metadata_value
                }),
                "item {} icon variants cannot duplicate selector {}={}",
                definition.id,
                variant.metadata_key,
                variant.metadata_value
            );
        }
        definition
            .name
            .validate(&format!("item {} name", definition.id));
        intern_item_id(&definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&ItemDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ItemDefinition> {
        self.definitions.values()
    }
}

pub(crate) fn display_name<'a>(
    item_id: &'a str,
    items: &'a ItemRegistry,
    blocks: &'a BlockRegistry,
    layers: &'a LayerRegistry,
    objects: &'a ObjectRegistry,
    tools: &'a ToolRegistry,
    language: Language,
) -> &'a str {
    if let Some(item) = items.get(item_id) {
        return item.name.text(language);
    }
    if let Some(block) = blocks.get(item_id) {
        return block.name.text(language);
    }
    if let Some(layer) = layers.get(item_id) {
        return layer.name.text(language);
    }
    if let Some(object) = objects.get(item_id) {
        return object.name.text(language);
    }
    if let Some(tool) = tools.get(item_id) {
        return tool.name.text(language);
    }
    item_id
}

pub(crate) struct ItemDisplayContext<'a> {
    pub(crate) items: &'a ItemRegistry,
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) layers: &'a LayerRegistry,
    pub(crate) objects: &'a ObjectRegistry,
    pub(crate) tools: &'a ToolRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) localization: &'a UiLocalization,
    pub(crate) language: Language,
}

pub(crate) fn display_name_with_metadata(
    item_id: &str,
    metadata: Option<(&str, &str)>,
    context: &ItemDisplayContext<'_>,
) -> String {
    let base_name = display_name(
        item_id,
        context.items,
        context.blocks,
        context.layers,
        context.objects,
        context.tools,
        context.language,
    );
    if item_id != BUCKET_TOOL_ID {
        return base_name.to_owned();
    }

    let contained_fluid = metadata
        .filter(|(key, _)| *key == BUCKET_FLUID_METADATA_KEY)
        .map(|(_, value)| value);
    let content_name = match contained_fluid {
        Some(fluid_definition_id) => context
            .fluids
            .id_of(fluid_definition_id)
            .and_then(|fluid_id| context.fluids.get(fluid_id))
            .map(|fluid| fluid.name.text(context.language))
            .unwrap_or(fluid_definition_id),
        None => context
            .localization
            .text(context.language, "inventory.bucket.empty"),
    };

    context.localization.format(
        context.language,
        "inventory.bucket.variant",
        &[("bucket", base_name), ("content", content_name)],
    )
}
