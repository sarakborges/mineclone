use bevy::prelude::*;

use crate::content::{biome::BiomeRegistry, color::Hsi};

use super::BiomeField;

impl BiomeField {
    pub fn grass_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Hsi {
        let sample = self.sample_surface(position);
        Hsi::blend_weighted(sample.influences.into_iter().map(|influence| {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            (biome.visuals.grass_color, influence.weight)
        }))
    }

    pub fn leaf_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Hsi {
        let sample = self.sample_surface(position);
        Hsi::blend_weighted(sample.influences.into_iter().map(|influence| {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            (biome.visuals.leaf_color, influence.weight)
        }))
    }

    pub fn foliage_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Hsi {
        let sample = self.sample_surface(position);
        Hsi::blend_weighted(sample.influences.into_iter().map(|influence| {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            (biome.visuals.foliage_color, influence.weight)
        }))
    }

    pub fn water_color(&self, position: Vec2, biomes: &BiomeRegistry, fallback: Hsi) -> Hsi {
        let sample = self.sample_surface(position);
        Hsi::blend_weighted(sample.influences.into_iter().map(|influence| {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            (
                biome.visuals.water_color.unwrap_or(fallback),
                influence.weight,
            )
        }))
    }
}
