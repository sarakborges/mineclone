use bevy::prelude::*;
use serde::Deserialize;

use super::{
    dimension::DimensionRegistry,
    registry::DefinitionMap,
    structure::StructureRegistry,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PortalCoordinateMapping {
    #[default]
    Exact,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PortalDestinationDefinition {
    pub(crate) dimension: String,
    #[serde(default)]
    pub(crate) coordinate_mapping: PortalCoordinateMapping,
    pub(crate) arrival_structure: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PortalDefinition {
    pub(crate) id: String,
    pub(crate) destination: PortalDestinationDefinition,
}

impl PortalDefinition {
    fn normalize_and_validate(&mut self) {
        self.id = self.id.trim().to_owned();
        self.destination.dimension = self.destination.dimension.trim().to_owned();
        self.destination.arrival_structure = self.destination.arrival_structure.trim().to_owned();

        assert!(!self.id.is_empty(), "portal id cannot be empty");
        assert!(
            !self.destination.dimension.is_empty(),
            "portal {} destination.dimension cannot be empty",
            self.id
        );
        assert!(
            !self.destination.arrival_structure.is_empty(),
            "portal {} destination.arrivalStructure cannot be empty",
            self.id
        );
    }

    pub(crate) fn validate_references(
        &self,
        dimensions: &DimensionRegistry,
        structures: &StructureRegistry,
    ) {
        assert!(
            dimensions.get(&self.destination.dimension).is_some(),
            "portal {} references missing destination dimension {}",
            self.id,
            self.destination.dimension
        );
        assert!(
            structures.resolves_reference(&self.destination.arrival_structure),
            "portal {} references missing arrival structure or structure group {}",
            self.id,
            self.destination.arrival_structure
        );
    }
}

#[derive(Resource, Default, Clone)]
pub(crate) struct PortalRegistry {
    definitions: DefinitionMap<PortalDefinition>,
}

impl PortalRegistry {
    pub(crate) fn insert(&mut self, mut definition: PortalDefinition) {
        definition.normalize_and_validate();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &PortalDefinition> {
        self.definitions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_definition_deserializes_exact_coordinate_arrival_structure() {
        let definition: PortalDefinition = serde_json::from_str(
            r#"{
                "id": "asteria:test_portal",
                "destination": {
                    "dimension": "asteria:the_umbral",
                    "coordinateMapping": "exact",
                    "arrivalStructure": "asteria:portal_arrival"
                }
            }"#,
        )
        .expect("portal definition should deserialize");

        assert_eq!(definition.id, "asteria:test_portal");
        assert_eq!(definition.destination.dimension, "asteria:the_umbral");
        assert_eq!(
            definition.destination.coordinate_mapping,
            PortalCoordinateMapping::Exact
        );
        assert_eq!(
            definition.destination.arrival_structure,
            "asteria:portal_arrival"
        );
    }

    #[test]
    fn exact_coordinate_mapping_is_the_default() {
        let definition: PortalDefinition = serde_json::from_str(
            r#"{
                "id": "asteria:test_portal",
                "destination": {
                    "dimension": "asteria:the_umbral",
                    "arrivalStructure": "asteria:portal_arrival"
                }
            }"#,
        )
        .expect("portal definition should deserialize");

        assert_eq!(
            definition.destination.coordinate_mapping,
            PortalCoordinateMapping::Exact
        );
    }
}
