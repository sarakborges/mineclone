use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountainPeakRadius {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum BiomeDistribution {
    #[default]
    Regional,
    MountainBelt {
        scale: f32,
        threshold: f32,
        width: f32,
        warp_scale: f32,
        warp_strength: f32,
    },
    MountainPeak {
        spacing: f32,
        chance: f32,
        radius: MountainPeakRadius,
        warp_scale: f32,
        warp_strength: f32,
    },
}

impl BiomeDistribution {
    pub fn is_regional(self) -> bool {
        matches!(self, Self::Regional)
    }

    pub fn validate(self, biome_id: &str) {
        match self {
            Self::Regional => {}
            Self::MountainBelt {
                scale,
                threshold,
                width,
                warp_scale,
                warp_strength,
            } => {
                assert!(
                    scale > 0.0,
                    "biome {biome_id} mountain belt scale must be positive"
                );
                assert!(
                    (0.0..=1.0).contains(&threshold),
                    "biome {biome_id} mountain belt threshold must be between 0 and 1"
                );
                assert!(
                    width > 0.0 && width <= 1.0,
                    "biome {biome_id} mountain belt width must be between 0 and 1"
                );
                assert!(
                    threshold >= width,
                    "biome {biome_id} mountain belt threshold must be greater than or equal to width"
                );
                assert!(
                    warp_scale > 0.0,
                    "biome {biome_id} mountain belt warpScale must be positive"
                );
                assert!(
                    warp_strength >= 0.0,
                    "biome {biome_id} mountain belt warpStrength cannot be negative"
                );
            }
            Self::MountainPeak {
                spacing,
                chance,
                radius,
                warp_scale,
                warp_strength,
            } => {
                assert!(
                    spacing > 0.0,
                    "biome {biome_id} mountain peak spacing must be positive"
                );
                assert!(
                    (0.0..=1.0).contains(&chance),
                    "biome {biome_id} mountain peak chance must be between 0 and 1"
                );
                assert!(
                    radius.min > 0.0,
                    "biome {biome_id} mountain peak radius.min must be positive"
                );
                assert!(
                    radius.max >= radius.min,
                    "biome {biome_id} mountain peak radius.max must be greater than or equal to min"
                );
                assert!(
                    warp_scale > 0.0,
                    "biome {biome_id} mountain peak warpScale must be positive"
                );
                assert!(
                    warp_strength >= 0.0,
                    "biome {biome_id} mountain peak warpStrength cannot be negative"
                );
            }
        }
    }
}
