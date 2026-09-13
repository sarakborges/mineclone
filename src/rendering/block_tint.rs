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
pub(crate) const DYE_VERTEX_COLOR_MARKER: f32 = 2.0;
const DYED_TINT_STRENGTH: f32 = 0.96;

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
    if let Some(dye) = secondary_property_dye_tint(block, cell, secondary_properties) {
        let dye = dye.to_srgba();
        return [
            dye.red + DYE_VERTEX_COLOR_MARKER,
            dye.green + DYE_VERTEX_COLOR_MARKER,
            dye.blue + DYE_VERTEX_COLOR_MARKER,
        ];
    }

    let base = base.to_srgba();
    [base.red, base.green, base.blue]
}

pub(crate) fn apply_secondary_property_tint(
    base: Color,
    block: &BlockDefinition,
    cell: VoxelCell,
    secondary_properties: &SecondaryPropertyRegistry,
) -> Color {
    let Some(dye) = secondary_property_dye_tint(block, cell, secondary_properties) else {
        return base;
    };

    let base = base.to_srgba();
    let dye = dye.to_srgba();
    let keep_base = 1.0 - DYED_TINT_STRENGTH;

    Color::srgba(
        base.red * keep_base + dye.red * DYED_TINT_STRENGTH,
        base.green * keep_base + dye.green * DYED_TINT_STRENGTH,
        base.blue * keep_base + dye.blue * DYED_TINT_STRENGTH,
        base.alpha,
    )
}
