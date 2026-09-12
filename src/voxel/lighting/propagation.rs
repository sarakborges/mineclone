use std::collections::HashSet;

use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
};
use crate::voxel::{
    coordinates::chunk_coord_from_world,
    light::VoxelLight,
    neighbors::CARDINAL_NEIGHBORS,
    world::VoxelWorld,
};

use super::{
    context::LightingContext,
    medium::{block_emission, light_filter, medium_dampening},
    queue::LightingQueue,
};

pub(super) fn relax(
    world: &mut VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
    queue: &mut LightingQueue,
) -> HashSet<IVec3> {
    relax_budgeted(
        world,
        blocks,
        fluids,
        secondary_properties,
        queue,
        usize::MAX,
    )
}

pub(super) fn relax_budgeted(
    world: &mut VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
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
        let desired = desired_light(
            world,
            blocks,
            fluids,
            secondary_properties,
            position,
            &mut context,
        );

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
    secondary_properties: &SecondaryPropertyRegistry,
    position: IVec3,
    context: &mut LightingContext,
) -> VoxelLight {
    let dampening = medium_dampening(world, blocks, fluids, position);
    let blocks_light = dampening >= VoxelLight::MAX_LEVEL;
    let attenuation = dampening.max(1);
    let filter = light_filter(world, blocks, secondary_properties, position);

    let sky = if blocks_light {
        [0; 3]
    } else {
        component_max(
            context.direct_sky_light(
                world,
                blocks,
                fluids,
                secondary_properties,
                position,
            ),
            filter_levels(
                propagated_neighbor_levels(world, position, attenuation, VoxelLight::sky_rgb),
                filter,
            ),
        )
    };

    let emitted = block_emission(world, blocks, secondary_properties, position);
    let block = if blocks_light {
        emitted
    } else {
        component_max(
            emitted,
            filter_levels(
                propagated_neighbor_levels(world, position, attenuation, VoxelLight::block_rgb),
                filter,
            ),
        )
    };

    VoxelLight::new_colored(sky, block)
}

fn propagated_neighbor_levels(
    world: &VoxelWorld,
    position: IVec3,
    attenuation: u8,
    channel: fn(VoxelLight) -> [u8; 3],
) -> [u8; 3] {
    let mut result = [0; 3];

    for direction in CARDINAL_NEIGHBORS {
        let incoming = channel(world.light_at(position + direction))
            .map(|level| level.saturating_sub(attenuation));
        result = component_max(result, incoming);
    }

    result
}

fn component_max(left: [u8; 3], right: [u8; 3]) -> [u8; 3] {
    [
        left[0].max(right[0]),
        left[1].max(right[1]),
        left[2].max(right[2]),
    ]
}

fn filter_levels(levels: [u8; 3], filter: [f32; 3]) -> [u8; 3] {
    [
        filtered_level(levels[0], filter[0]),
        filtered_level(levels[1], filter[1]),
        filtered_level(levels[2], filter[2]),
    ]
}

fn filtered_level(level: u8, factor: f32) -> u8 {
    (level as f32 * factor.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, VoxelLight::MAX_LEVEL as f32) as u8
}
