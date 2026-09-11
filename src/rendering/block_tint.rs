use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, block::BlockTint},
    world::biome_field::BiomeField,
};

pub(crate) fn block_tint_at(
    tint: BlockTint,
    position: Vec2,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
) -> Color {
    let color = match tint {
        BlockTint::None => return Color::WHITE,
        BlockTint::Grass => biome_field.grass_color(position, biomes),
        BlockTint::Leaf => biome_field.leaf_color(position, biomes),
        BlockTint::Foliage => biome_field.foliage_color(position, biomes),
    };

    Color::srgb(color.r, color.g, color.b)
}
