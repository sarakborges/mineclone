use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum BiomeDensityModifier {
    Cavern {
        carve_strength: f32,
        noise_scale: f32,
        openness: f32,
    },
    Solid {
        fill_strength: f32,
        noise_scale: f32,
        coverage: f32,
    },
}

impl BiomeDensityModifier {
    pub fn validate(&self, biome_id: &str) {
        match *self {
            Self::Cavern {
                carve_strength,
                noise_scale,
                openness,
            } => {
                assert!(
                    carve_strength >= 0.0,
                    "biome {biome_id} cavern carveStrength cannot be negative"
                );
                assert!(
                    noise_scale > 0.0,
                    "biome {biome_id} cavern noiseScale must be positive"
                );
                assert!(
                    (0.0..=1.0).contains(&openness),
                    "biome {biome_id} cavern openness must be between 0 and 1"
                );
            }
            Self::Solid {
                fill_strength,
                noise_scale,
                coverage,
            } => {
                assert!(
                    fill_strength >= 0.0,
                    "biome {biome_id} solid fillStrength cannot be negative"
                );
                assert!(
                    noise_scale > 0.0,
                    "biome {biome_id} solid noiseScale must be positive"
                );
                assert!(
                    (0.0..=1.0).contains(&coverage),
                    "biome {biome_id} solid coverage must be between 0 and 1"
                );
            }
        }
    }
}
