use bevy::prelude::*;

use crate::content::{biome::BiomeRegistry, color::Rgb};

use super::BiomeField;

impl BiomeField {
    pub fn grass_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Rgb {
        let sample = self.sample_surface(position);
        let mut color = Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };

        for influence in sample.influences {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let grass = biome.visuals.grass_color;

            color.r += grass.r * influence.weight;
            color.g += grass.g * influence.weight;
            color.b += grass.b * influence.weight;
        }

        color
    }

    pub fn leaf_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Rgb {
        let sample = self.sample_surface(position);
        let mut color = Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };

        for influence in sample.influences {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let leaf = biome.visuals.leaf_color;

            color.r += leaf.r * influence.weight;
            color.g += leaf.g * influence.weight;
            color.b += leaf.b * influence.weight;
        }

        color
    }

    pub fn foliage_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Rgb {
        let sample = self.sample_surface(position);
        let mut color = Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };

        for influence in sample.influences {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let foliage = biome.visuals.foliage_color;

            color.r += foliage.r * influence.weight;
            color.g += foliage.g * influence.weight;
            color.b += foliage.b * influence.weight;
        }

        color
    }

    pub fn water_color(&self, position: Vec2, biomes: &BiomeRegistry, fallback: Rgb) -> Rgb {
        let sample = self.sample_surface(position);
        let mut color = Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };

        for influence in sample.influences {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let water = biome.visuals.water_color.unwrap_or(fallback);

            color.r += water.r * influence.weight;
            color.g += water.g * influence.weight;
            color.b += water.b * influence.weight;
        }

        color
    }
}
