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
