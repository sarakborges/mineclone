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

pub(crate) fn secondary_property_dye_tint(
    block: &BlockDefinition,
    cell: VoxelCell,
    secondary_properties: &SecondaryPropertyRegistry,
) -> Option<Color> {
    if !block
        .secondary_properties
        .iter()
        .any(|property| property == DYED_PROPERTY_ID)
    {
        return None;
    }

    let value_id = cell.secondary_property(DYED_PROPERTY_ID)?;
    let dye = secondary_properties.get(DYED_PROPERTY_ID, value_id)?;

    Some(Color::srgb(dye.color.r, dye.color.g, dye.color.b))
}

pub(crate) fn block_vertex_tint(
    base: Color,
    block: &BlockDefinition,
    cell: VoxelCell,
    secondary_properties: &SecondaryPropertyRegistry,
) -> [f32; 3] {
    let tint = secondary_property_dye_tint(block, cell, secondary_properties).unwrap_or(base);
    let tint = tint.to_srgba();
    [tint.red, tint.green, tint.blue]
}

pub(crate) fn apply_secondary_property_tint(
    base: Color,
    block: &BlockDefinition,
    cell: VoxelCell,
    secondary_properties: &SecondaryPropertyRegistry,
) -> Color {
    secondary_property_dye_tint(block, cell, secondary_properties).unwrap_or(base)
}
