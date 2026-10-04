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
        block_state::{BlockState, BlockStateToken},
        cell::VoxelCell,
    },
    world::biome_field::BiomeField,
};

const DYE_SATURATION_GAMMA: f32 = 1.85;

pub(crate) fn block_tint_at(
    tint: BlockTint,
    _position: Vec2,
    _biome_field: &BiomeField,
    _biomes: &BiomeRegistry,
) -> Color {
    match tint {
        BlockTint::None | BlockTint::Grass | BlockTint::Leaf | BlockTint::Foliage => Color::WHITE,
    }
}

pub(crate) fn block_tint_for_biome(
    tint: BlockTint,
    _biome_id: &str,
    _biomes: &BiomeRegistry,
) -> Option<Color> {
    Some(match tint {
        BlockTint::None | BlockTint::Grass | BlockTint::Leaf | BlockTint::Foliage => Color::WHITE,
    })
}

pub(crate) fn block_tint_at_with_override(
    tint: BlockTint,
    position: Vec2,
    _biome_override: Option<&str>,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
) -> Color {
    block_tint_at(tint, position, biome_field, biomes)
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

    let value_id = cell.block_state().get_token(dyed_property_token())?;
    let dye = secondary_properties.get(DYED_PROPERTY_ID, value_id)?;
    if dye.color.intensity <= f32::EPSILON {
        return Some(Color::BLACK);
    }

    let saturation = 1.0 - (1.0 - dye.color.saturation.clamp(0.0, 1.0)).powf(DYE_SATURATION_GAMMA);
    Some(
        Hsi::new(dye.color.hue, saturation, dye.color.intensity)
            .normalized()
            .to_color(),
    )
}

fn dyed_property_token() -> BlockStateToken {
    static TOKEN: OnceLock<BlockStateToken> = OnceLock::new();
    *TOKEN.get_or_init(|| BlockState::token(DYED_PROPERTY_ID))
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
