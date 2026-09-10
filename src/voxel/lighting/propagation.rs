use std::collections::HashSet;

use bevy::prelude::*;

use crate::content::{block::BlockRegistry, fluid::FluidRegistry};
use crate::voxel::{
    chunk::CHUNK_SIZE, light::VoxelLight, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
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
    let mut changed_chunks = HashSet::new();
    let mut context = LightingContext::default();

    while let Some(position) = queue.pop() {
        if !world.is_loaded_at(position) {
            continue;
        }

        let current = world.light_at(position);
        let desired = desired_light(world, blocks, fluids, position, &mut context);

        if current == desired {
            continue;
        }

        world.set_light_at(position, desired);
        changed_chunks.insert(chunk_coord(position));
        queue.enqueue_neighbors(position);
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

fn chunk_coord(world_position: IVec3) -> IVec3 {
    let size = CHUNK_SIZE as i32;

    IVec3::new(
        world_position.x.div_euclid(size),
        world_position.y.div_euclid(size),
        world_position.z.div_euclid(size),
    )
}
