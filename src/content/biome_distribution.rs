use serde::Deserialize;

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
}

impl BiomeDistribution {
    pub fn is_regional(self) -> bool {
        matches!(self, Self::Regional)
    }

    pub fn validate(self, biome_id: &str) {
        let Self::MountainBelt {
            scale,
            threshold,
            width,
            warp_scale,
            warp_strength,
        } = self
        else {
            return;
        };

        assert!(scale > 0.0, "biome {biome_id} mountain belt scale must be positive");
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
}
