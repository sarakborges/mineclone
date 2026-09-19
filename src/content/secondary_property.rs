use bevy::{platform::collections::HashMap, prelude::*};
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{color::Hsi, registry::DefinitionMap};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryPropertyDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub color: Hsi,
}

#[derive(Resource, Default, Clone)]
pub struct SecondaryPropertyRegistry {
    definitions: HashMap<String, DefinitionMap<SecondaryPropertyDefinition>>,
}

impl SecondaryPropertyRegistry {
    pub fn insert(&mut self, property: String, definition: SecondaryPropertyDefinition) {
        assert!(
            !property.is_empty(),
            "secondary property group cannot have an empty id"
        );
        assert!(
            !definition.id.is_empty(),
            "secondary property {property} cannot contain a value with an empty id"
        );
        definition.name.validate(&format!(
            "secondary property {property}:{} name",
            definition.id
        ));
        assert!(
            definition.color.is_valid(),
            "secondary property {property}:{} HSI color must use finite hue and saturation/intensity between 0 and 1",
            definition.id
        );

        self.definitions
            .entry(property)
            .or_default()
            .insert(definition.id.clone(), definition);
    }

    pub fn contains_property(&self, property: &str) -> bool {
        self.definitions.contains_key(property)
    }

    pub fn get(&self, property: &str, id: &str) -> Option<&SecondaryPropertyDefinition> {
        self.definitions.get(property)?.get(id)
    }

    pub fn iter<'a>(
        &'a self,
        property: &str,
    ) -> impl Iterator<Item = &'a SecondaryPropertyDefinition> {
        self.definitions
            .get(property)
            .into_iter()
            .flat_map(|definitions| definitions.values())
    }
}
