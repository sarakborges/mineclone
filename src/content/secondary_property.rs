use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{color::Rgb, registry::DefinitionMap};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryPropertyDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub color: Rgb,
}

#[derive(Resource, Default)]
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
        for (channel, value) in [
            ("r", definition.color.r),
            ("g", definition.color.g),
            ("b", definition.color.b),
        ] {
            assert!(
                (0.0..=1.0).contains(&value),
                "secondary property {property}:{} color.{channel} must be between 0 and 1",
                definition.id
            );
        }

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
