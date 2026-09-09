use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeHydrology {
    #[serde(default = "default_true")]
    pub can_generate_lake: bool,
    #[serde(default = "default_true")]
    pub can_generate_river: bool,
    #[serde(default = "default_multiplier")]
    pub lake_chance_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub river_width_multiplier: f32,
}

impl Default for BiomeHydrology {
    fn default() -> Self {
        Self {
            can_generate_lake: true,
            can_generate_river: true,
            lake_chance_multiplier: 1.0,
            river_width_multiplier: 1.0,
        }
    }
}

impl BiomeHydrology {
    pub fn validate(&self, biome_id: &str) {
        assert!(
            self.lake_chance_multiplier >= 0.0,
            "biome {biome_id} hydrology.lakeChanceMultiplier cannot be negative"
        );
        assert!(
            self.river_width_multiplier >= 0.0,
            "biome {biome_id} hydrology.riverWidthMultiplier cannot be negative"
        );
    }
}

const fn default_true() -> bool {
    true
}

const fn default_multiplier() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydrology_defaults_allow_surface_water_features() {
        let hydrology = BiomeHydrology::default();

        assert!(hydrology.can_generate_lake);
        assert!(hydrology.can_generate_river);
        assert_eq!(hydrology.lake_chance_multiplier, 1.0);
        assert_eq!(hydrology.river_width_multiplier, 1.0);
    }
}
