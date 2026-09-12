use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
};
use crate::voxel::{light::VoxelLight, world::VoxelWorld};

const DYED_PROPERTY_ID: &str = "dyed";

pub(super) fn medium_dampening(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    position: IVec3,
) -> u8 {
    block_dampening(world, blocks, position).max(fluid_dampening(world, fluids, position))
}

pub(super) fn block_emission(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
    position: IVec3,
) -> [u8; 3] {
    let Some(cell) = world.cell_at(position) else {
        return [0; 3];
    };
    let Some(block) = blocks.get(cell.block_id) else {
        return [0; 3];
    };
    let level = block.light_emission.min(VoxelLight::MAX_LEVEL);
    if level == 0 {
        return [0; 3];
    }

    let Some(value_id) = cell.secondary_property(DYED_PROPERTY_ID) else {
        return [level; 3];
    };
    let Some(dye) = secondary_properties.get(DYED_PROPERTY_ID, value_id) else {
        return [level; 3];
    };

    [
        colored_level(level, dye.color.r),
        colored_level(level, dye.color.g),
        colored_level(level, dye.color.b),
    ]
}

pub(super) fn light_filter(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
    position: IVec3,
) -> [f32; 3] {
    let Some(cell) = world.cell_at(position) else {
        return [1.0; 3];
    };
    let Some(block) = blocks.get(cell.block_id) else {
        return [1.0; 3];
    };

    if block.light_dampening >= VoxelLight::MAX_LEVEL {
        return [1.0; 3];
    }
    if !block
        .secondary_properties
        .iter()
        .any(|property| property == DYED_PROPERTY_ID)
    {
        return [1.0; 3];
    }

    let Some(value_id) = cell.secondary_property(DYED_PROPERTY_ID) else {
        return [1.0; 3];
    };
    let Some(dye) = secondary_properties.get(DYED_PROPERTY_ID, value_id) else {
        return [1.0; 3];
    };

    [dye.color.r, dye.color.g, dye.color.b]
}

fn block_dampening(world: &VoxelWorld, blocks: &BlockRegistry, position: IVec3) -> u8 {
    let Some(block_id) = world.block_id_at(position) else {
        return 0;
    };

    blocks
        .get(block_id)
        .map(|block| block.light_dampening.min(VoxelLight::MAX_LEVEL))
        .unwrap_or(VoxelLight::MAX_LEVEL)
}

fn fluid_dampening(world: &VoxelWorld, fluids: &FluidRegistry, position: IVec3) -> u8 {
    let Some(cell) = world.fluid_at(position) else {
        return 0;
    };

    fluids
        .get(cell.fluid_id)
        .unwrap_or_else(|| panic!("missing fluid definition for id {}", cell.fluid_id))
        .light_dampening
        .min(VoxelLight::MAX_LEVEL)
}

fn colored_level(level: u8, factor: f32) -> u8 {
    (level as f32 * factor.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, VoxelLight::MAX_LEVEL as f32) as u8
}
