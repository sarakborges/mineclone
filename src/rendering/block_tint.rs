use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        block::{BlockDefinition, BlockTint},
        secondary_property::SecondaryPropertyRegistry,
    },
    voxel::cell::VoxelCell,
    world::biome_field::BiomeField,
};

const DYED_PROPERTY_ID: &str = "dyed";
const DYED_TINT_STRENGTH: f32 = 0.12;

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

pub(crate) fn apply_secondary_property_tint(
    base: Color,
    block: &BlockDefinition,
    cell: VoxelCell,
    secondary_properties: &SecondaryPropertyRegistry,
) -> Color {
    if !block
        .secondary_properties
        .iter()
        .any(|property| property == DYED_PROPERTY_ID)
    {
        return base;
    }

    let Some(value_id) = cell.secondary_property(DYED_PROPERTY_ID) else {
        return base;
    };
    let Some(dye) = secondary_properties.get(DYED_PROPERTY_ID, value_id) else {
        return base;
    };

    let base = base.to_srgba();
    let tint_r = 1.0 - DYED_TINT_STRENGTH + dye.color.r * DYED_TINT_STRENGTH;
    let tint_g = 1.0 - DYED_TINT_STRENGTH + dye.color.g * DYED_TINT_STRENGTH;
    let tint_b = 1.0 - DYED_TINT_STRENGTH + dye.color.b * DYED_TINT_STRENGTH;

    Color::srgba(
        base.red * tint_r,
        base.green * tint_g,
        base.blue * tint_b,
        base.alpha,
    )
}
