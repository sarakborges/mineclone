use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::registry::DefinitionMap;

const DEFAULT_BIOME_WEIGHT: f32 = 1.0;
const DEFAULT_REGION_MIN: u32 = 384;
const DEFAULT_REGION_MAX: u32 = 768;
const MAX_REGION_SPAN: u32 = 16_384;

/// One biome identity in the shared biome universe.
///
/// Spatial placement is capability-owned. A biome participates in the 2D
/// surface field only when `surfaceLayout` is authored. Future volume-layout
/// authoring extends this same identity instead of creating a parallel biome
/// type hierarchy.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeDefinition {
    pub id: String,
    pub name: LocalizedText,
    #[serde(default)]
    pub surface_layout: Option<SurfaceBiomeLayoutDefinition>,
}

/// Authored inputs owned exclusively by the 2D surface biome layout.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceBiomeLayoutDefinition {
    #[serde(default = "default_biome_weight")]
    pub weight: f32,
    #[serde(default)]
    pub region_size: BiomeRegionSize,
    #[serde(default)]
    pub cannot_border: Vec<String>,
}

impl Default for SurfaceBiomeLayoutDefinition {
    fn default() -> Self {
        Self {
            weight: default_biome_weight(),
            region_size: BiomeRegionSize::default(),
            cannot_border: Vec::new(),
        }
    }
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
        let Some(surface) = &self.surface_layout else {
            return;
        };
        let own_dimension = biome_dimension_components(&self.id)
            .expect("validated biome id must contain a dimension");
        for forbidden in &surface.cannot_border {
            assert!(
                forbidden != &self.id,
                "biome {} cannot forbid bordering itself",
                self.id
            );
            let target = biomes.get(forbidden).unwrap_or_else(|| {
                panic!(
                    "biome {} surfaceLayout.cannotBorder references missing biome {}",
                    self.id, forbidden
                )
            });
            let target_dimension = biome_dimension_components(&target.id)
                .expect("validated biome id must contain a dimension");
            assert_eq!(
                target_dimension, own_dimension,
                "biome {} surfaceLayout.cannotBorder target {} belongs to a different dimension",
                self.id, forbidden
            );
            assert!(
                target.surface_layout.is_some(),
                "biome {} surfaceLayout.cannotBorder target {} does not participate in the surface layout",
                self.id,
                forbidden
            );
        }
    }

    fn validate(&self) {
        assert_valid_biome_id(&self.id);
        self.name.validate(&format!("biome {} name", self.id));
        if let Some(surface) = &self.surface_layout {
            surface.validate(&self.id);
        }
    }
}

impl SurfaceBiomeLayoutDefinition {
    fn validate(&self, biome_id: &str) {
        assert!(
            self.weight.is_finite() && self.weight > 0.0,
            "biome {biome_id} surfaceLayout.weight must be finite and positive"
        );
        assert!(
            self.region_size.min > 0,
            "biome {biome_id} surfaceLayout.regionSize.min must be positive"
        );
        assert!(
            self.region_size.max >= self.region_size.min,
            "biome {biome_id} surfaceLayout.regionSize.max must be greater than or equal to min"
        );
        assert!(
            self.region_size.max <= MAX_REGION_SPAN,
            "biome {biome_id} surfaceLayout.regionSize.max must not exceed {MAX_REGION_SPAN} blocks"
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

    pub fn surface_for_dimension<'a>(
        &'a self,
        dimension_id: &'a str,
    ) -> impl Iterator<Item = &'a BiomeDefinition> + 'a {
        self.iter().filter(move |definition| {
            definition.surface_layout.is_some() && definition.belongs_to_dimension(dimension_id)
        })
    }
}

fn default_biome_weight() -> f32 {
    DEFAULT_BIOME_WEIGHT
}

fn assert_valid_biome_id(id: &str) {
    let Some((namespace, dimension)) = biome_dimension_components(id) else {
        panic!("biome id {id} must use the form <namespace>:<dimension>/<biome>");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(id: &str, surface: bool) -> BiomeDefinition {
        let mut value = serde_json::json!({
            "id": id,
            "name": {
                "english": "Biome",
                "portuguese_brazil": "Biome",
                "spanish": "Biome"
            }
        });
        if surface {
            value["surfaceLayout"] = serde_json::json!({});
        }
        serde_json::from_value(value).expect("biome definition must deserialize")
    }

    #[test]
    fn surface_layout_is_explicit_and_uses_forward_only_defaults() {
        let identity_only = definition("asteria:overworld/caverns", false);
        assert!(identity_only.surface_layout.is_none());

        let definition = definition("asteria:overworld/plains", true);
        let surface = definition
            .surface_layout
            .as_ref()
            .expect("surface layout must exist");
        assert_eq!(surface.weight, 1.0);
        assert_eq!(surface.region_size, BiomeRegionSize::default());
        assert!(surface.cannot_border.is_empty());
        assert!(definition.belongs_to_dimension("asteria:overworld"));
        assert!(!definition.belongs_to_dimension("asteria:umbral"));
    }

    #[test]
    fn surface_dimension_filter_excludes_identity_only_biomes() {
        let mut registry = BiomeRegistry::default();
        registry.insert(definition("asteria:overworld/caverns", false));
        registry.insert(definition("asteria:overworld/plains", true));
        registry.insert(definition("asteria:umbral/wraith_grove", true));

        let ids = registry
            .surface_for_dimension("asteria:overworld")
            .map(|definition| definition.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["asteria:overworld/plains"]);
    }
}
