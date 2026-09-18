use serde::Deserialize;

use super::{
    biome::{BiomeKind, BiomeRegistry},
    builtin_ids::WATER_FLUID_ID,
    fluid::FluidRegistry,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionHydrology {
    #[serde(default = "default_water_fluid")]
    pub water_fluid: String,
    #[serde(default)]
    pub ocean_biome: Option<String>,
    #[serde(default)]
    pub coast_biome: Option<String>,
    #[serde(default = "default_feature_weight")]
    pub river_weight: f32,
    #[serde(default = "default_feature_weight")]
    pub lake_weight: f32,
}

impl Default for DimensionHydrology {
    fn default() -> Self {
        Self {
            water_fluid: WATER_FLUID_ID.to_owned(),
            ocean_biome: None,
            coast_biome: None,
            river_weight: default_feature_weight(),
            lake_weight: default_feature_weight(),
        }
    }
}

impl DimensionHydrology {
    pub fn validate_references(
        &self,
        dimension_id: &str,
        biomes: &BiomeRegistry,
        fluids: &FluidRegistry,
    ) {
        assert!(
            fluids.id_of(&self.water_fluid).is_some(),
            "dimension {dimension_id} hydrology references missing waterFluid: {}",
            self.water_fluid
        );

        for (field, weight) in [
            ("riverWeight", self.river_weight),
            ("lakeWeight", self.lake_weight),
        ] {
            assert!(
                weight.is_finite() && weight >= 0.0,
                "dimension {dimension_id} hydrology.{field} must be finite and non-negative"
            );
        }

        for (field, biome_id) in [
            ("oceanBiome", self.ocean_biome.as_deref()),
            ("coastBiome", self.coast_biome.as_deref()),
        ] {
            let Some(biome_id) = biome_id else {
                continue;
            };
            let biome = biomes.get(biome_id).unwrap_or_else(|| {
                panic!("dimension {dimension_id} hydrology.{field} references missing biome: {biome_id}")
            });

            assert_eq!(
                biome.kind,
                BiomeKind::Surface,
                "dimension {dimension_id} hydrology.{field} must reference a surface biome: {biome_id}"
            );
        }

    }
}

fn default_water_fluid() -> String {
    WATER_FLUID_ID.to_owned()
}

fn default_feature_weight() -> f32 {
    1.0
}
