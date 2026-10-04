use bevy::prelude::*;

use crate::content::{biome::BiomeRegistry, builtin_ids::PLAINS_BIOME_ID, color::Hsi};

/// Temporary render-only boundary left after deleting the legacy biome field.
///
/// This type does not own biome layout, placement, transitions, terrain, or
/// search. It exists only so preserved rendering code can keep compiling until
/// Phase 3 provides the new authoritative biome-query capability.
#[derive(Clone, Default, Resource)]
pub struct BiomeField;

impl BiomeField {
    pub fn grass_color(&self, _position: Vec2, biomes: &BiomeRegistry) -> Hsi {
        biomes
            .get(PLAINS_BIOME_ID)
            .unwrap_or_else(|| panic!("missing fallback biome definition: {PLAINS_BIOME_ID}"))
            .visuals()
            .grass_color
    }

    pub fn leaf_color(&self, _position: Vec2, biomes: &BiomeRegistry) -> Hsi {
        biomes
            .get(PLAINS_BIOME_ID)
            .unwrap_or_else(|| panic!("missing fallback biome definition: {PLAINS_BIOME_ID}"))
            .visuals()
            .leaf_color
    }

    pub fn foliage_color(&self, _position: Vec2, biomes: &BiomeRegistry) -> Hsi {
        biomes
            .get(PLAINS_BIOME_ID)
            .unwrap_or_else(|| panic!("missing fallback biome definition: {PLAINS_BIOME_ID}"))
            .visuals()
            .foliage_color
    }

    pub fn water_color(&self, _position: Vec2, biomes: &BiomeRegistry, fallback: Hsi) -> Hsi {
        biomes
            .get(PLAINS_BIOME_ID)
            .unwrap_or_else(|| panic!("missing fallback biome definition: {PLAINS_BIOME_ID}"))
            .visuals()
            .water_color
            .unwrap_or(fallback)
    }
}
