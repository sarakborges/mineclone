use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
};
use crate::voxel::{light::VoxelLight, world::VoxelWorld};

const DYED_PROPERTY_ID: &str = "dyed";
const DYE_LIGHT_SATURATION_GAMMA: f32 = 1.85;

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

    if !block
        .secondary_properties
        .iter()
        .any(|property| property == DYED_PROPERTY_ID)
    {
        return [level; 3];
    }

    let Some(dye_id) = cell.secondary_property(DYED_PROPERTY_ID) else {
        return [level; 3];
    };
    let Some(dye) = secondary_properties.get(DYED_PROPERTY_ID, dye_id) else {
        return [level; 3];
    };

    let peak = dye.color.r.max(dye.color.g).max(dye.color.b);
    if peak <= f32::EPSILON {
        // Light fallback is always white, including invalid/black dye colors.
        return [level; 3];
    }

    let strengthen = |channel: f32| {
        (channel / peak)
            .clamp(0.0, 1.0)
            .powf(DYE_LIGHT_SATURATION_GAMMA)
    };

    [
        colored_emission_channel(level, strengthen(dye.color.r)),
        colored_emission_channel(level, strengthen(dye.color.g)),
        colored_emission_channel(level, strengthen(dye.color.b)),
    ]
}

pub(super) fn light_filter(
    _world: &VoxelWorld,
    _blocks: &BlockRegistry,
    _secondary_properties: &SecondaryPropertyRegistry,
    _position: IVec3,
) -> [f32; 3] {
    [1.0; 3]
}

fn colored_emission_channel(level: u8, factor: f32) -> u8 {
    (level as f32 * factor.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, VoxelLight::MAX_LEVEL as f32) as u8
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
