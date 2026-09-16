use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum BiomeTerrainModifier {
    Cliffs {
        scale: f32,
        threshold: f32,
        height: f32,
        edge_width: f32,
        warp_scale: f32,
        warp_strength: f32,
    },
}

impl BiomeTerrainModifier {
    pub fn maximum_height_offset(self) -> f32 {
        match self {
            Self::Cliffs { height, .. } => height.max(0.0),
        }
    }

    pub fn validate(self, biome_id: &str) {
        match self {
            Self::Cliffs {
                scale,
                threshold,
                height,
                edge_width,
                warp_scale,
                warp_strength,
            } => {
                assert!(scale > 0.0, "biome {biome_id} cliffs scale must be positive");
                assert!(
                    (0.0..=1.0).contains(&threshold),
                    "biome {biome_id} cliffs threshold must be between 0 and 1"
                );
                assert!(
                    height >= 0.0,
                    "biome {biome_id} cliffs height cannot be negative"
                );
                assert!(
                    edge_width > 0.0 && edge_width <= 1.0,
                    "biome {biome_id} cliffs edgeWidth must be between 0 and 1"
                );
                assert!(
                    warp_scale > 0.0,
                    "biome {biome_id} cliffs warpScale must be positive"
                );
                assert!(
                    warp_strength >= 0.0,
                    "biome {biome_id} cliffs warpStrength cannot be negative"
                );
            }
        }
    }
}
