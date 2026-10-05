use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::registry::DefinitionMap;

const DEFAULT_BIOME_WEIGHT: f32 = 1.0;
const DEFAULT_REGION_MIN: u32 = 384;
const DEFAULT_REGION_MAX: u32 = 768;
const MAX_REGION_SPAN: u32 = 16_384;
const DEFAULT_TERRAIN_BASE_HEIGHT_OFFSET: f32 = 8.0;
const DEFAULT_TERRAIN_MACRO_AMPLITUDE: f32 = 18.0;
const DEFAULT_TERRAIN_MACRO_SCALE: u32 = 640;
const DEFAULT_TERRAIN_DETAIL_AMPLITUDE: f32 = 4.0;
const DEFAULT_TERRAIN_DETAIL_SCALE: u32 = 96;
const MAX_TERRAIN_SCALE: u32 = 16_384;
const MAX_TERRAIN_AMPLITUDE: f32 = 512.0;
const MAX_TERRAIN_3D_VERTICAL_SPAN: i64 = 512;
const MAX_FLOATING_ROUGHNESS: f32 = 0.5;
const MAX_SURFACE_LAYER_DEPTH: u32 = 64;

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
    #[serde(default)]
    pub surface_terrain: Option<SurfaceTerrainDefinition>,
    #[serde(default)]
    pub surface_layers: Option<Vec<SurfaceLayerDefinition>>,
    #[serde(default)]
    pub terrain_3d: Option<Terrain3dDefinition>,
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

/// Authored continuous base-surface profile for one surface biome.
///
/// The profile is interpreted by the terrain owner. It does not own biome
/// placement, chunk generation, materials, caves, or 3D feature placement.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTerrainDefinition {
    #[serde(default = "default_terrain_base_height_offset")]
    pub base_height_offset: f32,
    #[serde(default = "default_terrain_macro_amplitude")]
    pub macro_amplitude: f32,
    #[serde(default = "default_terrain_macro_scale")]
    pub macro_scale: u32,
    #[serde(default = "default_terrain_detail_amplitude")]
    pub detail_amplitude: f32,
    #[serde(default = "default_terrain_detail_scale")]
    pub detail_scale: u32,
}

impl Default for SurfaceTerrainDefinition {
    fn default() -> Self {
        Self {
            base_height_offset: default_terrain_base_height_offset(),
            macro_amplitude: default_terrain_macro_amplitude(),
            macro_scale: default_terrain_macro_scale(),
            detail_amplitude: default_terrain_detail_amplitude(),
            detail_scale: default_terrain_detail_scale(),
        }
    }
}

/// Ordered generated solid-material layers for one surface biome.
///
/// Every finite layer owns `depth` voxels measured downward from the local
/// exposed terrain surface. The final depthless layer is the core material.
/// Terrain still owns solidity; these rules only classify already-solid voxels.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceLayerDefinition {
    pub block: String,
    #[serde(default)]
    pub depth: Option<u32>,
}

/// Optional true-3D terrain contributions authored by a biome.
///
/// These rules are consumed by the single terrain-density owner. They do not
/// create a second terrain field or redefine biome ownership.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Terrain3dDefinition {
    #[serde(default)]
    pub floating_formation: Option<FloatingFormationDefinition>,
}

/// Bounded additive floating mass authored in absolute world Y.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FloatingFormationDefinition {
    pub min_y: i32,
    pub max_y: i32,
    pub horizontal_scale: u32,
    pub detail_scale: u32,
    pub coverage: f32,
    pub roughness: f32,
    pub density_scale: f32,
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

    pub(crate) fn surface_terrain_profile(&self) -> SurfaceTerrainDefinition {
        self.surface_terrain.unwrap_or_default()
    }

    pub(crate) fn surface_layers_profile(&self) -> Option<&[SurfaceLayerDefinition]> {
        self.surface_layers.as_deref()
    }

    pub(crate) const fn terrain_3d_profile(&self) -> Option<Terrain3dDefinition> {
        self.terrain_3d
    }

    fn validate(&self) {
        assert_valid_biome_id(&self.id);
        self.name.validate(&format!("biome {} name", self.id));
        if let Some(surface) = &self.surface_layout {
            surface.validate(&self.id);
        }
        if let Some(terrain) = self.surface_terrain {
            terrain.validate(&self.id);
        }
        if let Some(layers) = &self.surface_layers {
            validate_surface_layers(&self.id, layers);
        }
        if let Some(terrain_3d) = self.terrain_3d {
            terrain_3d.validate(&self.id);
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

impl SurfaceTerrainDefinition {
    fn validate(self, biome_id: &str) {
        assert!(
            self.base_height_offset.is_finite(),
            "biome {biome_id} surfaceTerrain.baseHeightOffset must be finite"
        );
        assert!(
            self.macro_amplitude.is_finite()
                && (0.0..=MAX_TERRAIN_AMPLITUDE).contains(&self.macro_amplitude),
            "biome {biome_id} surfaceTerrain.macroAmplitude must be finite and within 0..={MAX_TERRAIN_AMPLITUDE}"
        );
        assert!(
            self.detail_amplitude.is_finite()
                && (0.0..=MAX_TERRAIN_AMPLITUDE).contains(&self.detail_amplitude),
            "biome {biome_id} surfaceTerrain.detailAmplitude must be finite and within 0..={MAX_TERRAIN_AMPLITUDE}"
        );
        assert!(
            (2..=MAX_TERRAIN_SCALE).contains(&self.macro_scale),
            "biome {biome_id} surfaceTerrain.macroScale must be within 2..={MAX_TERRAIN_SCALE}"
        );
        assert!(
            (2..=MAX_TERRAIN_SCALE).contains(&self.detail_scale),
            "biome {biome_id} surfaceTerrain.detailScale must be within 2..={MAX_TERRAIN_SCALE}"
        );
        assert!(
            self.detail_scale <= self.macro_scale,
            "biome {biome_id} surfaceTerrain.detailScale must not exceed macroScale"
        );
    }
}

fn validate_surface_layers(biome_id: &str, layers: &[SurfaceLayerDefinition]) {
    assert!(
        !layers.is_empty(),
        "biome {biome_id} surfaceLayers must contain at least one layer"
    );
    let mut finite_depth = 0_u32;
    for (index, layer) in layers.iter().enumerate() {
        let block = layer.block.trim();
        let valid_block_id = block.split_once(':').is_some_and(|(namespace, local)| {
            !namespace.is_empty() && !local.is_empty() && !local.contains(':')
        });
        assert!(
            valid_block_id && block == layer.block,
            "biome {biome_id} surfaceLayers[{index}].block must be a trimmed namespaced block id"
        );

        let final_layer = index + 1 == layers.len();
        match (final_layer, layer.depth) {
            (false, Some(depth)) => {
                assert!(
                    depth > 0,
                    "biome {biome_id} surfaceLayers[{index}].depth must be positive"
                );
                finite_depth = finite_depth
                    .checked_add(depth)
                    .expect("surface layer depth must fit u32");
                assert!(
                    finite_depth <= MAX_SURFACE_LAYER_DEPTH,
                    "biome {biome_id} finite surface layer depth must not exceed {MAX_SURFACE_LAYER_DEPTH} blocks"
                );
            }
            (false, None) => panic!(
                "biome {biome_id} surfaceLayers[{index}] requires depth before the final core layer"
            ),
            (true, None) => {}
            (true, Some(_)) => panic!(
                "biome {biome_id} final surfaceLayers entry is the core layer and must omit depth"
            ),
        }
    }
}

impl Terrain3dDefinition {
    fn validate(self, biome_id: &str) {
        if let Some(floating) = self.floating_formation {
            floating.validate(biome_id);
        }
    }
}

impl FloatingFormationDefinition {
    fn validate(self, biome_id: &str) {
        assert!(
            self.max_y > self.min_y,
            "biome {biome_id} terrain3d.floatingFormation.maxY must be greater than minY"
        );
        let vertical_span = i64::from(self.max_y) - i64::from(self.min_y);
        assert!(
            vertical_span <= MAX_TERRAIN_3D_VERTICAL_SPAN,
            "biome {biome_id} terrain3d.floatingFormation vertical span must not exceed {MAX_TERRAIN_3D_VERTICAL_SPAN} blocks"
        );
        assert!(
            (2..=MAX_TERRAIN_SCALE).contains(&self.horizontal_scale),
            "biome {biome_id} terrain3d.floatingFormation.horizontalScale must be within 2..={MAX_TERRAIN_SCALE}"
        );
        assert!(
            (2..=MAX_TERRAIN_SCALE).contains(&self.detail_scale),
            "biome {biome_id} terrain3d.floatingFormation.detailScale must be within 2..={MAX_TERRAIN_SCALE}"
        );
        assert!(
            self.detail_scale <= self.horizontal_scale,
            "biome {biome_id} terrain3d.floatingFormation.detailScale must not exceed horizontalScale"
        );
        assert!(
            self.coverage.is_finite() && self.coverage > 0.0 && self.coverage <= 1.0,
            "biome {biome_id} terrain3d.floatingFormation.coverage must be finite and within (0, 1]"
        );
        assert!(
            self.roughness.is_finite()
                && (0.0..=MAX_FLOATING_ROUGHNESS).contains(&self.roughness),
            "biome {biome_id} terrain3d.floatingFormation.roughness must be finite and within 0..={MAX_FLOATING_ROUGHNESS}"
        );
        assert!(
            self.density_scale.is_finite()
                && self.density_scale > 0.0
                && self.density_scale <= MAX_TERRAIN_AMPLITUDE,
            "biome {biome_id} terrain3d.floatingFormation.densityScale must be finite and within (0, {MAX_TERRAIN_AMPLITUDE}]"
        );
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

fn default_terrain_base_height_offset() -> f32 {
    DEFAULT_TERRAIN_BASE_HEIGHT_OFFSET
}

fn default_terrain_macro_amplitude() -> f32 {
    DEFAULT_TERRAIN_MACRO_AMPLITUDE
}

fn default_terrain_macro_scale() -> u32 {
    DEFAULT_TERRAIN_MACRO_SCALE
}

fn default_terrain_detail_amplitude() -> f32 {
    DEFAULT_TERRAIN_DETAIL_AMPLITUDE
}

fn default_terrain_detail_scale() -> u32 {
    DEFAULT_TERRAIN_DETAIL_SCALE
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
        assert_eq!(
            definition.surface_terrain_profile(),
            SurfaceTerrainDefinition::default()
        );
        assert!(definition.surface_layers_profile().is_none());
        assert!(definition.terrain_3d_profile().is_none());
        assert!(definition.belongs_to_dimension("asteria:overworld"));
        assert!(!definition.belongs_to_dimension("asteria:umbral"));
    }

    #[test]
    fn surface_layers_use_finite_layers_and_depthless_core() {
        let definition: BiomeDefinition = serde_json::from_value(serde_json::json!({
            "id": "asteria:overworld/plains",
            "name": {
                "english": "Plains",
                "portuguese_brazil": "Planicies",
                "spanish": "Llanuras"
            },
            "surfaceLayout": {},
            "surfaceLayers": [
                { "block": "asteria:grass_block", "depth": 1 },
                { "block": "asteria:dirt", "depth": 4 },
                { "block": "asteria:stone" }
            ]
        }))
        .expect("surface layers must deserialize");
        definition.validate();
        let layers = definition
            .surface_layers_profile()
            .expect("surface layers must be authored");
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].depth, Some(1));
        assert_eq!(layers[2].depth, None);
    }

    #[test]
    fn terrain_3d_floating_formation_is_explicit() {
        let definition: BiomeDefinition = serde_json::from_value(serde_json::json!({
            "id": "asteria:overworld/floating_islands",
            "name": {
                "english": "Floating Islands",
                "portuguese_brazil": "Ilhas Flutuantes",
                "spanish": "Islas Flotantes"
            },
            "surfaceLayout": {},
            "terrain3d": {
                "floatingFormation": {
                    "minY": 200,
                    "maxY": 280,
                    "horizontalScale": 112,
                    "detailScale": 40,
                    "coverage": 0.55,
                    "roughness": 0.18,
                    "densityScale": 28.0
                }
            }
        }))
        .expect("floating terrain definition must deserialize");
        let terrain_3d = definition
            .terrain_3d_profile()
            .expect("terrain3d must be authored");
        let floating = terrain_3d
            .floating_formation
            .expect("floating formation must be authored");
        assert_eq!(floating.min_y, 200);
        assert_eq!(floating.max_y, 280);
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
