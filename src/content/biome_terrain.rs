use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BiomeTerrain {
    Rolling {
        base_height: f32,
        amplitude: f32,
        scale: f32,
        detail_amplitude: f32,
        detail_scale: f32,
    },
    Mountains {
        base_height: f32,
        amplitude: f32,
        scale: f32,
        sharpness: f32,
    },
    Ocean {
        floor_depth: f32,
        amplitude: f32,
        scale: f32,
    },
}

impl BiomeTerrain {
    pub fn maximum_height_offset(&self) -> f32 {
        match *self {
            Self::Rolling {
                base_height,
                amplitude,
                detail_amplitude,
                ..
            } => base_height + amplitude.abs() + detail_amplitude.abs(),
            Self::Mountains {
                base_height,
                amplitude,
                ..
            } => base_height + amplitude.abs(),
            Self::Ocean {
                floor_depth,
                amplitude,
                ..
            } => -floor_depth + amplitude.abs(),
        }
    }

    pub fn validate(&self, biome_id: &str) {
        match *self {
            Self::Rolling {
                amplitude,
                scale,
                detail_amplitude,
                detail_scale,
                ..
            } => {
                assert!(amplitude >= 0.0, "biome {biome_id} rolling amplitude cannot be negative");
                assert!(scale > 0.0, "biome {biome_id} rolling scale must be positive");
                assert!(
                    detail_amplitude >= 0.0,
                    "biome {biome_id} rolling detail_amplitude cannot be negative"
                );
                assert!(
                    detail_scale > 0.0,
                    "biome {biome_id} rolling detail_scale must be positive"
                );
            }
            Self::Mountains {
                amplitude,
                scale,
                sharpness,
                ..
            } => {
                assert!(
                    amplitude >= 0.0,
                    "biome {biome_id} mountains amplitude cannot be negative"
                );
                assert!(scale > 0.0, "biome {biome_id} mountains scale must be positive");
                assert!(
                    sharpness > 0.0,
                    "biome {biome_id} mountains sharpness must be positive"
                );
            }
            Self::Ocean {
                floor_depth,
                amplitude,
                scale,
            } => {
                assert!(
                    floor_depth >= 0.0,
                    "biome {biome_id} ocean floor_depth cannot be negative"
                );
                assert!(
                    amplitude >= 0.0,
                    "biome {biome_id} ocean amplitude cannot be negative"
                );
                assert!(scale > 0.0, "biome {biome_id} ocean scale must be positive");
            }
        }
    }
}
