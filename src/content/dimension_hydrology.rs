use serde::Deserialize;

use super::{builtin_ids::WATER_FLUID_ID, fluid::FluidRegistry};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionHydrology {
    #[serde(default = "default_water_fluid")]
    pub water_fluid: String,
    #[serde(default = "default_feature_weight")]
    pub river_weight: f32,
    #[serde(default = "default_feature_weight")]
    pub lake_weight: f32,
}

impl Default for DimensionHydrology {
    fn default() -> Self {
        Self {
            water_fluid: WATER_FLUID_ID.to_owned(),
            river_weight: default_feature_weight(),
            lake_weight: default_feature_weight(),
        }
    }
}

impl DimensionHydrology {
    pub fn validate_references(&self, dimension_id: &str, fluids: &FluidRegistry) {
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

    }
}

fn default_water_fluid() -> String {
    WATER_FLUID_ID.to_owned()
}

fn default_feature_weight() -> f32 {
    1.0
}
