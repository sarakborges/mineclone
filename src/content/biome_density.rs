use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
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
    FloatingIsland {
        fill_margin: f32,
        noise_scale: f32,
        edge_irregularity: f32,
        top_roughness: f32,
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
            Self::FloatingIsland {
                fill_margin,
                noise_scale,
                edge_irregularity,
                top_roughness,
            } => {
                assert!(
                    fill_margin.is_finite() && fill_margin > 0.0,
                    "biome {biome_id} floating island fillMargin must be positive and finite"
                );
                assert!(
                    noise_scale.is_finite() && noise_scale > 0.0,
                    "biome {biome_id} floating island noiseScale must be positive and finite"
                );
                assert!(
                    edge_irregularity.is_finite()
                        && (0.0..=0.4).contains(&edge_irregularity),
                    "biome {biome_id} floating island edgeIrregularity must be between 0 and 0.4"
                );
                assert!(
                    top_roughness.is_finite() && (0.0..=0.4).contains(&top_roughness),
                    "biome {biome_id} floating island topRoughness must be between 0 and 0.4"
                );
            }
        }
    }
}
