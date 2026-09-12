use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
};
use crate::voxel::{light::VoxelLight, world::VoxelWorld};

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
    _secondary_properties: &SecondaryPropertyRegistry,
    position: IVec3,
) -> [u8; 3] {
    let Some(block_id) = world.block_id_at(position) else {
        return [0; 3];
    };

    let level = blocks
        .get(block_id)
        .map(|block| block.light_emission.min(VoxelLight::MAX_LEVEL))
        .unwrap_or(0);

    [level; 3]
}

pub(super) fn light_filter(
    _world: &VoxelWorld,
    _blocks: &BlockRegistry,
    _secondary_properties: &SecondaryPropertyRegistry,
    _position: IVec3,
) -> [f32; 3] {
    [1.0; 3]
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
