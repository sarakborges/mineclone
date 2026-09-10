use bevy::prelude::*;

use crate::content::{
    biome::{BiomeRegistry, BiomeUnderwaterTint},
    color::Rgb,
};

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

    pub fn underwater_tint(&self, position: Vec3, biomes: &BiomeRegistry) -> BiomeUnderwaterTint {
        let sample = self.sample_resolved(position);
        let mut color = Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        let mut opacity = 0.0;

        for influence in sample.influences {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let tint = biome.visuals.underwater_tint;

            color.r += tint.color.r * influence.weight;
            color.g += tint.color.g * influence.weight;
            color.b += tint.color.b * influence.weight;
            opacity += tint.opacity * influence.weight;
        }

        BiomeUnderwaterTint { color, opacity }
    }
}
