use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
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
    Gorge {
        base_height: f32,
        depth: f32,
        wall_height: f32,
    },
    Alps {
        base_height: f32,
        amplitude: f32,
        scale: f32,
        sharpness: f32,
        detail_amplitude: f32,
        detail_scale: f32,
    },
    MountainBelt {
        base_height: f32,
        amplitude: f32,
        scale: f32,
        sharpness: f32,
        detail_amplitude: f32,
        detail_scale: f32,
    },
    Volcano {
        base_height: f32,
        height: f32,
        crater_depth: f32,
        crater_radius: f32,
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
            Self::Gorge {
                base_height,
                wall_height,
                ..
            } => base_height + wall_height.max(0.0),
            Self::Alps {
                base_height,
                amplitude,
                detail_amplitude,
                ..
            }
            | Self::MountainBelt {
                base_height,
                amplitude,
                detail_amplitude,
                ..
            } => base_height + amplitude.abs() + detail_amplitude.abs(),
            Self::Volcano {
                base_height,
                height,
                ..
            } => base_height + height.max(0.0),
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
                assert!(
                    amplitude >= 0.0,
                    "biome {biome_id} rolling amplitude cannot be negative"
                );
                assert!(
                    scale > 0.0,
                    "biome {biome_id} rolling scale must be positive"
                );
                assert!(
                    detail_amplitude >= 0.0,
                    "biome {biome_id} rolling detailAmplitude cannot be negative"
                );
                assert!(
                    detail_scale > 0.0,
                    "biome {biome_id} rolling detailScale must be positive"
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
                assert!(
                    scale > 0.0,
                    "biome {biome_id} mountains scale must be positive"
                );
                assert!(
                    sharpness > 0.0,
                    "biome {biome_id} mountains sharpness must be positive"
                );
            },
            Self::Gorge {
                depth,
                wall_height,
                ..
            } => {
                assert!(depth >= 0.0, "biome {biome_id} gorge depth cannot be negative");
                assert!(
                    wall_height >= 0.0,
                    "biome {biome_id} gorge wallHeight cannot be negative"
                );
            }
            Self::Alps {
                amplitude,
                scale,
                sharpness,
                detail_amplitude,
                detail_scale,
                ..
            } => {
                assert!(amplitude >= 0.0, "biome {biome_id} alps amplitude cannot be negative");
                assert!(scale > 0.0, "biome {biome_id} alps scale must be positive");
                assert!(sharpness > 0.0, "biome {biome_id} alps sharpness must be positive");
                assert!(
                    detail_amplitude >= 0.0,
                    "biome {biome_id} alps detailAmplitude cannot be negative"
                );
                assert!(
                    detail_scale > 0.0,
                    "biome {biome_id} alps detailScale must be positive"
                );
            }
            Self::MountainBelt {
                amplitude,
                scale,
                sharpness,
                detail_amplitude,
                detail_scale,
                ..
            } => {
                assert!(
                    amplitude >= 0.0,
                    "biome {biome_id} mountainBelt amplitude cannot be negative"
                );
                assert!(
                    scale > 0.0,
                    "biome {biome_id} mountainBelt scale must be positive"
                );
                assert!(
                    sharpness > 0.0,
                    "biome {biome_id} mountainBelt sharpness must be positive"
                );
                assert!(
                    detail_amplitude >= 0.0,
                    "biome {biome_id} mountainBelt detailAmplitude cannot be negative"
                );
                assert!(
                    detail_scale > 0.0,
                    "biome {biome_id} mountainBelt detailScale must be positive"
                );
            }
            Self::Volcano {
                height,
                crater_depth,
                crater_radius,
                ..
            } => {
                assert!(height > 0.0, "biome {biome_id} volcano height must be positive");
                assert!(
                    crater_depth >= 0.0,
                    "biome {biome_id} volcano craterDepth cannot be negative"
                );
                assert!(
                    (0.0..1.0).contains(&crater_radius),
                    "biome {biome_id} volcano craterRadius must be between 0 and 1"
                );
            }
        }
    }
}
