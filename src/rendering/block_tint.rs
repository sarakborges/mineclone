use std::sync::OnceLock;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        block::{BlockDefinition, BlockTint},
        builtin_ids::DYED_PROPERTY_ID,
        color::Hsi,
        secondary_property::SecondaryPropertyRegistry,
    },
    voxel::{
        cell::VoxelCell,
        secondary_properties::{SecondaryProperties, SecondaryPropertyToken},
    },
    world::biome_field::BiomeField,
};

const DYE_SATURATION_GAMMA: f32 = 1.85;

pub(crate) fn block_tint_at(
    tint: BlockTint,
    position: Vec2,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
) -> Color {
    match tint {
        BlockTint::None => Color::WHITE,
        BlockTint::Grass => biome_field.grass_color(position, biomes).to_color(),
        BlockTint::Leaf => biome_field.leaf_color(position, biomes).to_color(),
        BlockTint::Foliage => biome_field.foliage_color(position, biomes).to_color(),
    }
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

    let value_id = cell
        .secondary_properties()
        .get_token(dyed_property_token())?;
    let dye = secondary_properties.get(DYED_PROPERTY_ID, value_id)?;
    if dye.color.intensity <= f32::EPSILON {
        return Some(Color::BLACK);
    }

    let saturation =
        1.0 - (1.0 - dye.color.saturation.clamp(0.0, 1.0)).powf(DYE_SATURATION_GAMMA);
    Some(
        Hsi::new(dye.color.hue, saturation, dye.color.intensity)
            .normalized()
            .to_color(),
    )
}

fn dyed_property_token() -> SecondaryPropertyToken {
    static TOKEN: OnceLock<SecondaryPropertyToken> = OnceLock::new();
    *TOKEN.get_or_init(|| SecondaryProperties::token(DYED_PROPERTY_ID))
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
