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
        0
    } else {
        let transmission = filter[0].max(filter[1]).max(filter[2]);
        context
            .direct_sky_light(
                world,
                blocks,
                fluids,
                secondary_properties,
                position,
            )
            .max(filtered_level(
                propagated_neighbor_sky(world, position, attenuation),
                transmission,
            ))
    };

    let emitted = block_emission(world, blocks, secondary_properties, position);
    let block = if blocks_light {
        emitted
    } else {
        component_max(
            emitted,
            filter_levels(
                propagated_neighbor_block(world, position, attenuation),
                filter,
            ),
        )
    };

    VoxelLight::new_colored([sky; 3], block)
}

fn propagated_neighbor_sky(world: &VoxelWorld, position: IVec3, attenuation: u8) -> u8 {
    let mut result = 0;

    for direction in CARDINAL_NEIGHBORS {
        let incoming = world
            .light_at(position + direction)
            .sky()
            .saturating_sub(attenuation);
        result = result.max(incoming);
    }

    result
}

fn propagated_neighbor_block(
    world: &VoxelWorld,
    position: IVec3,
    attenuation: u8,
) -> [u8; 3] {
    let mut result = [0; 3];

    for direction in CARDINAL_NEIGHBORS {
        let incoming = attenuate_colored(
            world.light_at(position + direction).block_rgb(),
            attenuation,
        );
        result = component_max(result, incoming);
    }

    result
}

fn attenuate_colored(levels: [u8; 3], attenuation: u8) -> [u8; 3] {
    let peak = levels[0].max(levels[1]).max(levels[2]);
    if peak == 0 {
        return [0; 3];
    }

    let next_peak = peak.saturating_sub(attenuation);
    if next_peak == 0 {
        return [0; 3];
    }

    // Block-light distance is carried by the peak channel. Scale every channel
    // by the same ratio instead of subtracting the attenuation from R/G/B
    // independently. Independent subtraction made weak channels disappear first,
    // so colored light became progressively more saturated the farther it
    // travelled. Shared scaling keeps the hue approximately stable while the
    // overall intensity falls.
    levels.map(|level| {
        let scaled = level as u16 * next_peak as u16;
        let rounded = scaled + peak as u16 / 2;
        (rounded / peak as u16).min(next_peak as u16) as u8
    })
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

#[cfg(test)]
mod tests {
    use super::attenuate_colored;

    #[test]
    fn colored_attenuation_preserves_relative_channels() {
        assert_eq!(attenuate_colored([15, 3, 2], 1), [14, 3, 2]);
        assert_eq!(attenuate_colored([3, 4, 15], 1), [3, 4, 14]);
    }

    #[test]
    fn colored_attenuation_reduces_peak_by_medium_cost() {
        assert_eq!(attenuate_colored([15, 8, 4], 2), [13, 7, 3]);
        assert_eq!(attenuate_colored([2, 1, 1], 2), [0, 0, 0]);
    }

    #[test]
    fn white_light_stays_white_while_fading() {
        assert_eq!(attenuate_colored([15, 15, 15], 1), [14, 14, 14]);
        assert_eq!(attenuate_colored([7, 7, 7], 3), [4, 4, 4]);
    }
}
