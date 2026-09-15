use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceCarverRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum BiomeSurfaceCarver {
    Tunnel {
        spacing: f32,
        chance: f32,
        length: SurfaceCarverRange,
        radius: SurfaceCarverRange,
        elevation: SurfaceCarverRange,
        jitter: f32,
    },
}

impl BiomeSurfaceCarver {
    pub fn validate(self, biome_id: &str) {
        match self {
            Self::Tunnel {
                spacing,
                chance,
                length,
                radius,
                elevation,
                jitter,
            } => {
                assert!(spacing > 0.0, "biome {biome_id} tunnel spacing must be positive");
                assert!(
                    (0.0..=1.0).contains(&chance),
                    "biome {biome_id} tunnel chance must be between 0 and 1"
                );
                validate_range(biome_id, "tunnel length", length, false);
                validate_range(biome_id, "tunnel radius", radius, false);
                validate_range(biome_id, "tunnel elevation", elevation, true);
                assert!(jitter >= 0.0, "biome {biome_id} tunnel jitter cannot be negative");
                assert!(
                    jitter * 2.0 < spacing,
                    "biome {biome_id} tunnel jitter must be less than half its spacing"
                );
            }
        }
    }
}

fn validate_range(
    biome_id: &str,
    label: &str,
    range: SurfaceCarverRange,
    allow_zero: bool,
) {
    if allow_zero {
        assert!(range.min >= 0.0, "biome {biome_id} {label}.min cannot be negative");
    } else {
        assert!(range.min > 0.0, "biome {biome_id} {label}.min must be positive");
    }
    assert!(
        range.max >= range.min,
        "biome {biome_id} {label}.max must be greater than or equal to min"
    );
}
