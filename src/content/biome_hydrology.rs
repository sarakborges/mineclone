use serde::Deserialize;

#[derive(Clone, Copy, Debug)]
pub struct BiomeHydrologyRules {
    pub can_generate_lake: bool,
    pub can_generate_river: bool,
    pub lake_chance_multiplier: f32,
    pub river_width_multiplier: f32,
}

impl Default for BiomeHydrologyRules {
    fn default() -> Self {
        Self {
            can_generate_lake: true,
            can_generate_river: true,
            lake_chance_multiplier: 1.0,
            river_width_multiplier: 1.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
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
    #[serde(default)]
    pub ocean_bed_block: Option<String>,
    #[serde(default)]
    pub river_bed_block: Option<String>,
    #[serde(default)]
    pub lake_bed_block: Option<String>,
    #[serde(default)]
    pub shore_block: Option<String>,
}

impl Default for BiomeHydrology {
    fn default() -> Self {
        Self {
            can_generate_lake: true,
            can_generate_river: true,
            lake_chance_multiplier: 1.0,
            river_width_multiplier: 1.0,
            ocean_bed_block: None,
            river_bed_block: None,
            lake_bed_block: None,
            shore_block: None,
        }
    }
}

impl BiomeHydrology {
    pub fn rules(&self) -> BiomeHydrologyRules {
        BiomeHydrologyRules {
            can_generate_lake: self.can_generate_lake,
            can_generate_river: self.can_generate_river,
            lake_chance_multiplier: self.lake_chance_multiplier,
            river_width_multiplier: self.river_width_multiplier,
        }
    }

    pub fn validate(&self, biome_id: &str) {
        assert!(
            self.lake_chance_multiplier >= 0.0,
            "biome {biome_id} hydrology.lakeChanceMultiplier cannot be negative"
        );
        assert!(
            self.river_width_multiplier >= 0.0,
            "biome {biome_id} hydrology.riverWidthMultiplier cannot be negative"
        );

        for (field, block) in [
            ("oceanBedBlock", self.ocean_bed_block.as_deref()),
            ("riverBedBlock", self.river_bed_block.as_deref()),
            ("lakeBedBlock", self.lake_bed_block.as_deref()),
            ("shoreBlock", self.shore_block.as_deref()),
        ] {
            if let Some(block) = block {
                assert!(
                    !block.trim().is_empty(),
                    "biome {biome_id} hydrology.{field} cannot be empty"
                );
            }
        }
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

    #[test]
    fn hydrology_uses_readable_camel_case_json_fields() {
        let hydrology: BiomeHydrology = serde_json::from_str(
            r#"{
                "canGenerateLake": false,
                "canGenerateRiver": true,
                "lakeChanceMultiplier": 0.25,
                "riverWidthMultiplier": 1.5,
                "riverBedBlock": "asteria:sand",
                "shoreBlock": "asteria:gravel"
            }"#,
        )
        .unwrap();

        assert!(!hydrology.can_generate_lake);
        assert!(hydrology.can_generate_river);
        assert_eq!(hydrology.lake_chance_multiplier, 0.25);
        assert_eq!(hydrology.river_width_multiplier, 1.5);
        assert_eq!(hydrology.river_bed_block.as_deref(), Some("asteria:sand"));
        assert_eq!(hydrology.shore_block.as_deref(), Some("asteria:gravel"));
    }
}
