use serde::Deserialize;

use super::{
    biome::{BiomeKind, BiomeRegistry},
    block::BlockRegistry,
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
    #[serde(default)]
    pub ocean_bed_block: Option<String>,
    #[serde(default)]
    pub river_bed_block: Option<String>,
    #[serde(default)]
    pub lake_bed_block: Option<String>,
    #[serde(default)]
    pub shore_block: Option<String>,
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
            ocean_bed_block: None,
            river_bed_block: None,
            lake_bed_block: None,
            shore_block: None,
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
        blocks: &BlockRegistry,
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
                BiomeKind::Hydrology,
                "dimension {dimension_id} hydrology.{field} must reference a hydrology biome: {biome_id}"
            );
        }

        for (field, block_id) in [
            ("oceanBedBlock", self.ocean_bed_block.as_deref()),
            ("riverBedBlock", self.river_bed_block.as_deref()),
            ("lakeBedBlock", self.lake_bed_block.as_deref()),
            ("shoreBlock", self.shore_block.as_deref()),
        ] {
            let Some(block_id) = block_id else {
                continue;
            };

            assert!(
                blocks.get(block_id).is_some(),
                "dimension {dimension_id} hydrology.{field} references missing block: {block_id}"
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
