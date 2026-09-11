use std::collections::HashSet;

use bevy::prelude::*;

use crate::content::{block::BlockRegistry, fluid::FluidRegistry};
use crate::voxel::{
    coordinates::chunk_coord_from_world,
    light::VoxelLight,
    neighbors::CARDINAL_NEIGHBORS,
    world::VoxelWorld,
};

use super::{
    context::LightingContext,
    medium::{block_emission, medium_dampening},
    queue::LightingQueue,
};

pub(super) fn relax(
    world: &mut VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    queue: &mut LightingQueue,
) -> HashSet<IVec3> {
    relax_budgeted(world, blocks, fluids, queue, usize::MAX)
}

pub(super) fn relax_budgeted(
    world: &mut VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    queue: &mut LightingQueue,
    max_voxels: usize,
) -> HashSet<IVec3> {
    let mut changed_chunks = HashSet::new();
    let mut context = LightingContext::default();

    for _ in 0..max_voxels {
        let Some(position) = queue.pop() else {
            break;
        };

        if !world.is_loaded_at(position) {
            continue;
        }

        let current = world.light_at(position);
        let desired = desired_light(world, blocks, fluids, position, &mut context);

        if current == desired {
            continue;
        }

        world.set_light_at(position, desired);
        changed_chunks.insert(chunk_coord_from_world(position));
        queue.enqueue_with_neighbors(position);
    }

    changed_chunks
}

fn desired_light(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    position: IVec3,
    context: &mut LightingContext,
) -> VoxelLight {
    let dampening = medium_dampening(world, blocks, fluids, position);
    let blocks_light = dampening >= VoxelLight::MAX_LEVEL;
    let attenuation = dampening.max(1);

    let sky = if blocks_light {
        0
    } else {
        context
            .direct_sky_level(world, blocks, fluids, position)
            .max(propagated_neighbor_level(
                world,
                position,
                attenuation,
                VoxelLight::sky,
            ))
    };

    let emitted = block_emission(world, blocks, position);
    let block = if blocks_light {
        emitted
    } else {
        emitted.max(propagated_neighbor_level(
            world,
            position,
            attenuation,
            VoxelLight::block,
        ))
    };

    VoxelLight::new(sky, block)
}

fn propagated_neighbor_level(
    world: &VoxelWorld,
    position: IVec3,
    attenuation: u8,
    channel: fn(VoxelLight) -> u8,
) -> u8 {
    CARDINAL_NEIGHBORS
        .into_iter()
        .map(|direction| channel(world.light_at(position + direction)).saturating_sub(attenuation))
        .max()
        .unwrap_or(0)
}
