use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::registry::DefinitionMap;

const DEFAULT_BIOME_WEIGHT: f32 = 1.0;
const DEFAULT_REGION_MIN: u32 = 384;
const DEFAULT_REGION_MAX: u32 = 768;
const MAX_REGION_SPAN: u32 = 16_384;

/// Authored surface-biome layout inputs.
///
/// Terrain, materials, Structures, visuals, and other later phases deliberately
/// do not live here. Phase 3 owns only spatial biome-layout semantics.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeDefinition {
    pub id: String,
    pub name: LocalizedText,
    #[serde(default = "default_biome_weight")]
    pub weight: f32,
    #[serde(default)]
    pub region_size: BiomeRegionSize,
    #[serde(default)]
    pub cannot_border: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BiomeRegionSize {
    pub min: u32,
    pub max: u32,
}

impl Default for BiomeRegionSize {
    fn default() -> Self {
        Self {
            min: DEFAULT_REGION_MIN,
            max: DEFAULT_REGION_MAX,
        }
    }
}

impl BiomeDefinition {
    pub fn belongs_to_dimension(&self, dimension_id: &str) -> bool {
        biome_dimension_components(&self.id).is_some_and(|(namespace, dimension)| {
            dimension_components(dimension_id).is_some_and(
                |(dimension_namespace, dimension_name)| {
                    namespace == dimension_namespace && dimension == dimension_name
                },
            )
        })
    }

    pub fn validate_references(&self, biomes: &BiomeRegistry) {
        for forbidden in &self.cannot_border {
            assert!(
                forbidden != &self.id,
                "biome {} cannot forbid bordering itself",
                self.id
            );
            let target = biomes.get(forbidden).unwrap_or_else(|| {
                panic!(
                    "biome {} cannotBorder references missing biome {}",
                    self.id, forbidden
                )
            });
            assert!(
                target.belongs_to_dimension(
                    dimension_id_from_biome_id(&self.id)
                        .expect("validated biome id must contain a dimension")
                ),
                "biome {} cannotBorder target {} belongs to a different dimension",
                self.id,
                forbidden
            );
        }
    }

    fn validate(&self) {
        assert_valid_biome_id(&self.id);
        self.name.validate(&format!("biome {} name", self.id));
        assert!(
            self.weight.is_finite() && self.weight > 0.0,
            "biome {} weight must be finite and positive",
            self.id
        );
        assert!(
            self.region_size.min > 0,
            "biome {} regionSize.min must be positive",
            self.id
        );
        assert!(
            self.region_size.max >= self.region_size.min,
            "biome {} regionSize.max must be greater than or equal to min",
            self.id
        );
        assert!(
            self.region_size.max <= MAX_REGION_SPAN,
            "biome {} regionSize.max must not exceed {} blocks",
            self.id,
            MAX_REGION_SPAN
        );
        for forbidden in &self.cannot_border {
            assert_valid_biome_id(forbidden);
        }
    }
}

#[derive(Clone, Resource, Default)]
pub struct BiomeRegistry {
    definitions: DefinitionMap<BiomeDefinition>,
}

impl BiomeRegistry {
    pub fn insert(&mut self, definition: BiomeDefinition) {
        definition.validate();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BiomeDefinition> {
        self.definitions.values()
    }

    pub fn for_dimension<'a>(
        &'a self,
        dimension_id: &'a str,
    ) -> impl Iterator<Item = &'a BiomeDefinition> + 'a {
        self.iter()
            .filter(move |definition| definition.belongs_to_dimension(dimension_id))
    }
}

fn default_biome_weight() -> f32 {
    DEFAULT_BIOME_WEIGHT
}

fn assert_valid_biome_id(id: &str) {
    let Some((namespace, dimension)) = biome_dimension_components(id) else {
        panic!(
            "biome id {id} must use the form <namespace>:<dimension>/<biome>"
        );
    };
    assert!(!namespace.is_empty(), "biome id namespace cannot be empty");
    assert!(!dimension.is_empty(), "biome id dimension cannot be empty");
}

fn biome_dimension_components(id: &str) -> Option<(&str, &str)> {
    let (namespace, remainder) = id.split_once(':')?;
    let (dimension, local_name) = remainder.split_once('/')?;
    if namespace.is_empty() || dimension.is_empty() || local_name.is_empty() {
        return None;
    }
    Some((namespace, dimension))
}

fn dimension_components(id: &str) -> Option<(&str, &str)> {
    let (namespace, dimension) = id.split_once(':')?;
    if namespace.is_empty() || dimension.is_empty() || dimension.contains('/') {
        return None;
    }
    Some((namespace, dimension))
}

fn dimension_id_from_biome_id(id: &str) -> Option<String> {
    let (namespace, dimension) = biome_dimension_components(id)?;
    Some(format!("{namespace}:{dimension}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(id: &str) -> BiomeDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": {
                "english": "Biome",
                "portuguese_brazil": "Biome",
                "spanish": "Biome"
            }
        }))
        .expect("biome definition must deserialize")
    }

    #[test]
    fn layout_defaults_are_forward_only_and_valid() {
        let definition = definition("asteria:overworld/plains");
        assert_eq!(definition.weight, 1.0);
        assert_eq!(definition.region_size, BiomeRegionSize::default());
        assert!(definition.cannot_border.is_empty());
        assert!(definition.belongs_to_dimension("asteria:overworld"));
        assert!(!definition.belongs_to_dimension("asteria:umbral"));
    }

    #[test]
    fn dimension_filter_uses_biome_identity_namespace() {
        let mut registry = BiomeRegistry::default();
        registry.insert(definition("asteria:overworld/plains"));
        registry.insert(definition("asteria:umbral/wraith_grove"));

        let ids = registry
            .for_dimension("asteria:umbral")
            .map(|definition| definition.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["asteria:umbral/wraith_grove"]);
    }
}
