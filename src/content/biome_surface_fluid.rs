use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum BiomeSurfaceFluid {
    VolcanoCrater {
        fluid: String,
        minimum_strength: f32,
        level_offset: f32,
        spill_minimum_strength: f32,
        spill_maximum_strength: f32,
        spill_scale: f32,
        spill_width: f32,
        spill_level: u8,
    },
}

impl BiomeSurfaceFluid {
    pub fn fluid_id(&self) -> &str {
        match self {
            Self::VolcanoCrater { fluid, .. } => fluid,
        }
    }

    pub fn validate(&self, biome_id: &str) {
        match self {
            Self::VolcanoCrater {
                fluid,
                minimum_strength,
                level_offset,
                spill_minimum_strength,
                spill_maximum_strength,
                spill_scale,
                spill_width,
                spill_level,
            } => {
                assert!(
                    !fluid.trim().is_empty(),
                    "biome {biome_id} volcano crater fluid cannot be empty"
                );
                assert!(
                    (0.0..=1.0).contains(minimum_strength),
                    "biome {biome_id} volcano crater minimumStrength must be between 0 and 1"
                );
                assert!(
                    level_offset.is_finite() && *level_offset >= 0.0,
                    "biome {biome_id} volcano crater levelOffset must be finite and non-negative"
                );
                assert!(
                    (0.0..=1.0).contains(spill_minimum_strength),
                    "biome {biome_id} volcano crater spillMinimumStrength must be between 0 and 1"
                );
                assert!(
                    (0.0..=1.0).contains(spill_maximum_strength)
                        && spill_maximum_strength >= spill_minimum_strength,
                    "biome {biome_id} volcano crater spillMaximumStrength must be between spillMinimumStrength and 1"
                );
                assert!(
                    *spill_scale > 0.0 && spill_scale.is_finite(),
                    "biome {biome_id} volcano crater spillScale must be positive and finite"
                );
                assert!(
                    (0.0..=1.0).contains(spill_width),
                    "biome {biome_id} volcano crater spillWidth must be between 0 and 1"
                );
                assert!(
                    (1..=crate::voxel::fluid::MAX_FLUID_LEVEL).contains(spill_level),
                    "biome {biome_id} volcano crater spillLevel must be between 1 and {}",
                    crate::voxel::fluid::MAX_FLUID_LEVEL
                );
            }
        }
    }
}
