use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry, color::Hsi, fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
};
use crate::voxel::{
    cell::VoxelCell,
    fluid::FluidCell,
    light::{BlockLight, VoxelLight},
    world::VoxelWorld,
};

const DYED_PROPERTY_ID: &str = "dyed";
const DYE_LIGHT_SATURATION_GAMMA: f32 = 1.85;

pub(super) fn medium_dampening_for_cells(
    cell: Option<VoxelCell>,
    fluid: Option<FluidCell>,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> u8 {
    block_dampening(cell, blocks).max(fluid_dampening(fluid, fluids))
}

pub(super) fn block_emission_for_cell(
    cell: Option<VoxelCell>,
    blocks: &BlockRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
) -> BlockLight {
    let Some(cell) = cell else {
        return BlockLight::DARK;
    };
    let Some(block) = blocks.get(cell.block_id) else {
        return BlockLight::DARK;
    };

    let level = block.light_emission.min(VoxelLight::MAX_LEVEL);
    if level == 0 {
        return BlockLight::DARK;
    }

    if !block
        .secondary_properties
        .iter()
        .any(|property| property == DYED_PROPERTY_ID)
    {
        return BlockLight::new(0, 0, level);
    }

    let Some(dye_id) = cell.secondary_property(DYED_PROPERTY_ID) else {
        return BlockLight::new(0, 0, level);
    };
    let Some(dye) = secondary_properties.get(DYED_PROPERTY_ID, dye_id) else {
        return BlockLight::new(0, 0, level);
    };
    if dye.color.intensity <= f32::EPSILON {
        return BlockLight::new(0, 0, level);
    }

    let saturation =
        1.0 - (1.0 - dye.color.saturation.clamp(0.0, 1.0)).powf(DYE_LIGHT_SATURATION_GAMMA);
    BlockLight::from_hsi(
        Hsi::new(dye.color.hue, saturation, dye.color.intensity),
        level,
    )
}

pub(super) fn light_transmission(
    _world: &VoxelWorld,
    _blocks: &BlockRegistry,
    _secondary_properties: &SecondaryPropertyRegistry,
    _position: IVec3,
) -> f32 {
    1.0
}

fn block_dampening(cell: Option<VoxelCell>, blocks: &BlockRegistry) -> u8 {
    let Some(cell) = cell else {
        return 0;
    };

    let full_dampening = blocks
        .get(cell.block_id)
        .map(|block| block.light_dampening.min(VoxelLight::MAX_LEVEL))
        .unwrap_or(VoxelLight::MAX_LEVEL);

    crate::voxel::microblock::MicroblockMask::from_cell(cell).light_dampening(full_dampening)
}

fn fluid_dampening(cell: Option<FluidCell>, fluids: &FluidRegistry) -> u8 {
    let Some(cell) = cell else {
        return 0;
    };

    fluids
        .get(cell.fluid_id)
        .unwrap_or_else(|| panic!("missing fluid definition for id {}", cell.fluid_id))
        .light_dampening
        .min(VoxelLight::MAX_LEVEL)
}
