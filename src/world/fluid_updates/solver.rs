use bevy::prelude::*;

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        coordinates::chunk_coord_from_world,
        fluid::{FluidCell, MAX_FLUID_LEVEL},
        neighbors::{CARDINAL_NEIGHBORS, HORIZONTAL_NEIGHBORS},
        world::VoxelWorld,
    },
};

use crate::world::chunk_remesh::ChunkRemeshQueue;

pub(super) fn desired_fluid(
    world: &VoxelWorld,
    position: IVec3,
    current: Option<FluidCell>,
    fluids: &FluidRegistry,
) -> Option<FluidCell> {
    if world.is_solid(position) {
        return None;
    }

    if current.is_some_and(FluidCell::is_source) {
        return current;
    }

    if let Some(above) = world.fluid_at(position + IVec3::Y) {
        return Some(FluidCell::flowing(above.fluid_id, MAX_FLUID_LEVEL));
    }

    let target_supported = fluid_has_support(world, position);
    let mut strongest: Option<(u8, u16, FluidId)> = None;

    for offset in HORIZONTAL_NEIGHBORS {
        let neighbor_position = position + offset;
        let Some(neighbor) = world.fluid_at(neighbor_position) else {
            continue;
        };
        if !target_supported && !can_spill_over_edge(world, neighbor_position, neighbor) {
            continue;
        }

        let definition = fluids
            .get(neighbor.fluid_id)
            .unwrap_or_else(|| panic!("missing fluid definition for id {}", neighbor.fluid_id));
        let Some((level, spread_distance)) =
            horizontal_spread_state(neighbor, definition.max_spread)
        else {
            continue;
        };

        let candidate = (level, spread_distance, neighbor.fluid_id);
        if strongest.is_none_or(|current| {
            candidate.0 > current.0
                || (candidate.0 == current.0 && candidate.1 < current.1)
                || (candidate.0 == current.0
                    && candidate.1 == current.1
                    && candidate.2 < current.2)
        }) {
            strongest = Some(candidate);
        }
    }

    strongest.map(|(level, spread_distance, fluid_id)| {
        FluidCell::spreading(fluid_id, level, spread_distance)
    })
}

fn fluid_has_support(world: &VoxelWorld, position: IVec3) -> bool {
    if position.y == 0 {
        return true;
    }

    let below = position - IVec3::Y;
    world.is_solid(below) || world.fluid_at(below).is_some()
}

fn can_spill_over_edge(world: &VoxelWorld, position: IVec3, fluid: FluidCell) -> bool {
    if position.y == 0 || world.is_solid(position - IVec3::Y) {
        return true;
    }

    fluid.is_source() && world.fluid_at(position + IVec3::Y).is_none()
}

fn horizontal_spread_state(neighbor: FluidCell, max_spread: u16) -> Option<(u8, u16)> {
    let spread_distance = neighbor.spread_distance().saturating_add(1);
    if spread_distance > max_spread {
        return None;
    }

    let level = neighbor.level.saturating_sub(1).max(1);
    Some((level, spread_distance))
}

pub(super) fn enqueue_remesh(position: IVec3, remesh_queue: &mut ChunkRemeshQueue) {
    let center = chunk_coord_from_world(position);
    remesh_queue.enqueue_priority(center);

    for offset in CARDINAL_NEIGHBORS {
        let neighbor = chunk_coord_from_world(position + offset);
        if neighbor != center {
            remesh_queue.enqueue(neighbor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::chunk::VoxelChunk;

    #[test]
    fn horizontal_spread_respects_data_driven_range_independently_from_level() {
        let source = FluidCell::source(0, 2);
        assert_eq!(horizontal_spread_state(source, 0), None);
        assert_eq!(horizontal_spread_state(source, 1), Some((1, 1)));

        let first = FluidCell::spreading(0, 1, 1);
        assert_eq!(horizontal_spread_state(first, 1), None);
        assert_eq!(horizontal_spread_state(first, 2), Some((1, 2)));

        let far = FluidCell::spreading(0, 1, 20);
        assert_eq!(horizontal_spread_state(far, 20), None);
        assert_eq!(horizontal_spread_state(far, 21), Some((1, 21)));
    }

    #[test]
    fn source_water_spills_only_from_the_top_of_an_unsupported_column() {
        let position = IVec3::new(4, 10, 4);
        let source = FluidCell::source(0, MAX_FLUID_LEVEL);
        let mut exposed_world = VoxelWorld::default();
        exposed_world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());

        assert!(can_spill_over_edge(&exposed_world, position, source));
        assert!(!can_spill_over_edge(
            &exposed_world,
            position,
            FluidCell::flowing(0, MAX_FLUID_LEVEL),
        ));

        let mut covered_world = VoxelWorld::default();
        covered_world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());
        covered_world.set_fluid_at(position + IVec3::Y, Some(source));

        assert!(!can_spill_over_edge(&covered_world, position, source));
    }
}
