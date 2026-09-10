mod context;
mod medium;
mod propagation;
mod queue;

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use bevy::prelude::*;

use crate::content::{block::BlockRegistry, fluid::FluidRegistry};

use self::{propagation::relax, queue::LightingQueue};
use super::{chunk::CHUNK_SIZE, world::VoxelWorld};

pub(crate) fn initialize_chunk_lighting(
    world: &mut VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    if !world.clear_chunk_light(coord) {
        return HashSet::new();
    }

    let mut queue = LightingQueue::default();
    let origin = coord * CHUNK_SIZE as i32;
    queue.enqueue_chunk_voxels(origin);
    queue.enqueue_chunk_boundary_neighbors(origin);

    relax(world, blocks, fluids, &mut queue)
}

pub(crate) fn relight_after_voxel_edit(
    world: &mut VoxelWorld,
    position: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    let mut queue = LightingQueue::default();
    queue.enqueue(position);
    queue.enqueue_neighbors(position);

    relax(world, blocks, fluids, &mut queue)
}

pub(crate) fn relight_after_chunk_unloads(
    world: &mut VoxelWorld,
    unloaded: &[IVec3],
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    let mut queue = LightingQueue::default();
    let chunk_size = CHUNK_SIZE as i32;

    for coord in unloaded {
        queue.enqueue_chunk_boundary_neighbors(*coord * chunk_size);
    }

    relax(world, blocks, fluids, &mut queue)
}
