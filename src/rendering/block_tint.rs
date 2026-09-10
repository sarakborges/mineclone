use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, builtin_ids::GRASS_BLOCK_ID},
    world::biome_field::BiomeField,
};

pub(crate) fn block_tint_at(
    block_id: &str,
    position: Vec2,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
) -> Color {
    if block_id != GRASS_BLOCK_ID {
        return Color::WHITE;
    }

    let grass = biome_field.grass_color(position, biomes);
    Color::srgb(grass.r, grass.g, grass.b)
}

pub(crate) fn block_tint_with_opacity(
    block_id: &str,
    position: Vec2,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
    opacity: f32,
) -> Color {
    let tint = block_tint_at(block_id, position, biome_field, biomes).to_srgba();

    Color::srgba(tint.red, tint.green, tint.blue, opacity.clamp(0.0, 1.0))
}
