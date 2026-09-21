use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{registry::DefinitionMap, structure::StructureRegistry};

const DEFAULT_SET_ATTEMPTS: u32 = 24;

fn default_one_u32() -> u32 {
    1
}

fn default_chance() -> f32 {
    1.0
}

fn default_relative_to() -> String {
    "origin".to_owned()
}

fn default_attempts() -> u32 {
    DEFAULT_SET_ATTEMPTS
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureSetCount {
    #[serde(default = "default_one_u32")]
    pub min: u32,
    #[serde(default = "default_one_u32")]
    pub max: u32,
}

impl Default for StructureSetCount {
    fn default() -> Self {
        Self { min: 1, max: 1 }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureSetElementPlacement {
    #[serde(default = "default_relative_to")]
    pub relative_to: String,
    #[serde(default)]
    pub min_distance: u32,
    #[serde(default)]
    pub max_distance: u32,
    #[serde(default)]
    pub min_separation: u32,
    #[serde(default = "default_attempts")]
    pub attempts: u32,
    #[serde(default)]
    pub allow_overlap: bool,
}

impl Default for StructureSetElementPlacement {
    fn default() -> Self {
        Self {
            relative_to: default_relative_to(),
            min_distance: 0,
            max_distance: 0,
            min_separation: 0,
            attempts: DEFAULT_SET_ATTEMPTS,
            allow_overlap: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureSetElement {
    pub id: String,
    pub structure: String,
    #[serde(default)]
    pub count: StructureSetCount,
    #[serde(default = "default_chance")]
    pub chance: f32,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub placement: StructureSetElementPlacement,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureSetDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub locatable: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub conflict_groups: Vec<String>,
    #[serde(default)]
    pub reserve_space: bool,
    pub elements: Vec<StructureSetElement>,
}

impl StructureSetDefinition {
    fn validate(&self) {
        assert!(!self.id.trim().is_empty(), "structure set id cannot be empty");
        self.name
            .validate(&format!("structure set {} name", self.id));
        assert!(
            !self.elements.is_empty(),
            "structure set {} must define at least one element",
            self.id
        );

        for (index, group) in self.conflict_groups.iter().enumerate() {
            assert!(
                !group.trim().is_empty(),
                "structure set {} conflictGroups cannot contain empty values",
                self.id
            );
            assert!(
                !self.conflict_groups[..index].contains(group),
                "structure set {} conflictGroups cannot contain duplicates",
                self.id
            );
        }

        let mut known_elements = HashSet::new();
        for element in &self.elements {
            assert!(
                !element.id.trim().is_empty(),
                "structure set {} element id cannot be empty",
                self.id
            );
            assert!(
                known_elements.insert(element.id.as_str()),
                "structure set {} element id {} is duplicated",
                self.id,
                element.id
            );
            assert!(
                !element.structure.trim().is_empty(),
                "structure set {} element {} must reference a structure",
                self.id,
                element.id
            );
            assert!(
                element.count.min <= element.count.max,
                "structure set {} element {} count.min cannot exceed count.max",
                self.id,
                element.id
            );
            assert!(
                element.chance.is_finite() && (0.0..=1.0).contains(&element.chance),
                "structure set {} element {} chance must be between 0 and 1",
                self.id,
                element.id
            );
            assert!(
                element.placement.min_distance <= element.placement.max_distance,
                "structure set {} element {} minDistance cannot exceed maxDistance",
                self.id,
                element.id
            );
            assert!(
                element.placement.max_distance <= i32::MAX as u32,
                "structure set {} element {} maxDistance is too large",
                self.id,
                element.id
            );
            assert!(
                element.placement.min_separation <= i32::MAX as u32,
                "structure set {} element {} minSeparation is too large",
                self.id,
                element.id
            );
            assert!(
                element.placement.attempts > 0,
                "structure set {} element {} attempts must be positive",
                self.id,
                element.id
            );

            let relative_to = element.placement.relative_to.trim();
            assert!(
                !relative_to.is_empty(),
                "structure set {} element {} relativeTo cannot be empty",
                self.id,
                element.id
            );
            if relative_to != "origin" && relative_to != "any" {
                assert!(
                    known_elements.contains(relative_to),
                    "structure set {} element {} relativeTo must reference origin, any, or an earlier element; missing {}",
                    self.id,
                    element.id,
                    relative_to
                );
                assert_ne!(
                    relative_to,
                    element.id,
                    "structure set {} element {} cannot be relative to itself",
                    self.id,
                    element.id
                );
            }
        }
    }

    pub(crate) fn validate_references(&self, structures: &StructureRegistry) {
        for element in &self.elements {
            assert!(
                structures.resolves_reference(&element.structure),
                "structure set {} element {} references missing structure or structure group: {}",
                self.id,
                element.id,
                element.structure
            );
        }
    }

    pub(crate) fn horizontal_bounds(
        &self,
        structures: &StructureRegistry,
    ) -> Option<(IVec2, IVec2)> {
        let mut element_radii = HashMap::<&str, i32>::new();
        let mut maximum_prior_radius = 0_i32;
        let mut bounds: Option<(IVec2, IVec2)> = None;

        for element in &self.elements {
            let base_radius = match element.placement.relative_to.as_str() {
                "origin" => 0,
                "any" => maximum_prior_radius,
                reference => *element_radii.get(reference)?,
            };
            let radius = base_radius
                .checked_add(i32::try_from(element.placement.max_distance).ok()?)?;
            let (structure_minimum, structure_maximum) =
                structures.bounds_for_reference(&element.structure)?;
            let minimum = IVec2::splat(-radius) + structure_minimum;
            let maximum = IVec2::splat(radius) + structure_maximum;

            bounds = Some(match bounds {
                Some((current_minimum, current_maximum)) => (
                    current_minimum.min(minimum),
                    current_maximum.max(maximum),
                ),
                None => (minimum, maximum),
            });
            element_radii.insert(element.id.as_str(), radius);
            maximum_prior_radius = maximum_prior_radius.max(radius);
        }

        bounds
    }

    pub(crate) fn references_structure(
        &self,
        structure_id: &str,
        structures: &StructureRegistry,
    ) -> bool {
        self.elements.iter().any(|element| {
            structures.reference_contains_structure(&element.structure, structure_id)
        })
    }
}

#[derive(Clone, Resource, Default)]
pub struct StructureSetRegistry {
    definitions: DefinitionMap<StructureSetDefinition>,
}

impl StructureSetRegistry {
    pub fn insert(&mut self, definition: StructureSetDefinition) {
        definition.validate();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&StructureSetDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StructureSetDefinition> {
        self.definitions.values()
    }

    pub(crate) fn horizontal_bounds(
        &self,
        id: &str,
        structures: &StructureRegistry,
    ) -> Option<(IVec2, IVec2)> {
        self.get(id)?.horizontal_bounds(structures)
    }
}
