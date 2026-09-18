use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        cell::VoxelCell,
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
    target_cell: Option<VoxelCell>,
    current: Option<FluidCell>,
    fluids: &FluidRegistry,
) -> Option<FluidCell> {
    if target_cell.is_some() {
        return None;
    }

    if current.is_some_and(FluidCell::is_source) {
        return current;
    }

    if let Some(above) = world.fluid_at(position + IVec3::Y) {
        // Falling vertically starts a fresh horizontal run once the column lands.
        return Some(FluidCell::spreading(
            above.fluid_id,
            MAX_FLUID_LEVEL,
            0,
        ));
    }

    let mut strongest: Option<(u8, u16, FluidId)> = None;

    for offset in HORIZONTAL_NEIGHBORS {
        let neighbor_position = position + offset;
        let Some(neighbor) = world.fluid_at(neighbor_position) else {
            continue;
        };
        if !can_spread_horizontally_from(world, neighbor_position, neighbor) {
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

        let remaining_steps = definition
            .max_spread
            .saturating_sub(neighbor.spread_distance());
        if !horizontal_spread_is_preferred(
            world,
            neighbor_position,
            position,
            neighbor.fluid_id,
            remaining_steps,
        ) {
            continue;
        }

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

fn can_spread_horizontally_from(
    world: &VoxelWorld,
    position: IVec3,
    fluid: FluidCell,
) -> bool {
    if position.y == 0 || world.is_solid(position - IVec3::Y) {
        return true;
    }

    // A source at the exposed top of a fluid column may spill over an edge.
    // Dynamic falling cells remain vertical until they reach solid support.
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

fn horizontal_spread_is_preferred(
    world: &VoxelWorld,
    origin: IVec3,
    target: IVec3,
    fluid_id: FluidId,
    remaining_steps: u16,
) -> bool {
    let Some(preferred) =
        preferred_horizontal_directions(world, origin, fluid_id, remaining_steps)
    else {
        // No reachable drop within this horizontal run: spread normally.
        return true;
    };

    horizontal_direction_bit(target - origin)
        .is_some_and(|direction| preferred & direction != 0)
}

/// Returns the first-step directions that reach the nearest downward opening
/// within the fluid's remaining horizontal range. `None` means there is no
/// reachable drop, so normal radial spreading should be used instead.
fn preferred_horizontal_directions(
    world: &VoxelWorld,
    origin: IVec3,
    fluid_id: FluidId,
    remaining_steps: u16,
) -> Option<u8> {
    if remaining_steps == 0 {
        return None;
    }

    let mut queue = VecDeque::<(IVec3, u16)>::new();
    let mut visited = HashMap::<IVec3, (u16, u8)>::new();
    visited.insert(origin, (0, 0));

    for (index, offset) in HORIZONTAL_NEIGHBORS.into_iter().enumerate() {
        let position = origin + offset;
        if !can_flow_horizontally_through(world, position, fluid_id) {
            continue;
        }

        let direction = 1_u8 << index;
        visited.insert(position, (1, direction));
        queue.push_back((position, 1));
    }

    let mut nearest_drop = None;
    let mut preferred = 0_u8;

    while let Some((position, distance)) = queue.pop_front() {
        if nearest_drop.is_some_and(|best| distance > best) {
            break;
        }

        let direction_mask = visited
            .get(&position)
            .map(|(_, directions)| *directions)
            .expect("queued fluid path node must be visited");

        if can_fall_from(world, position) {
            match nearest_drop {
                None => {
                    nearest_drop = Some(distance);
                    preferred = direction_mask;
                }
                Some(best) if best == distance => {
                    preferred |= direction_mask;
                }
                Some(_) => {}
            }
            continue;
        }

        if distance >= remaining_steps || nearest_drop.is_some() {
            continue;
        }

        let next_distance = distance + 1;
        for offset in HORIZONTAL_NEIGHBORS {
            let next = position + offset;
            if !can_flow_horizontally_through(world, next, fluid_id) {
                continue;
            }

            match visited.get_mut(&next) {
                Some((known_distance, known_directions)) if *known_distance == next_distance => {
                    let merged = *known_directions | direction_mask;
                    if merged != *known_directions {
                        *known_directions = merged;
                        queue.push_back((next, next_distance));
                    }
                }
                Some(_) => {}
                None => {
                    visited.insert(next, (next_distance, direction_mask));
                    queue.push_back((next, next_distance));
                }
            }
        }
    }

    nearest_drop.map(|_| preferred)
}

fn can_flow_horizontally_through(
    world: &VoxelWorld,
    position: IVec3,
    fluid_id: FluidId,
) -> bool {
    world.sample_at(position).is_some_and(|(cell, fluid, _)| {
        cell.is_none() && fluid.is_none_or(|fluid| fluid.fluid_id == fluid_id)
    })
}

fn can_fall_from(world: &VoxelWorld, position: IVec3) -> bool {
    if position.y == 0 {
        return false;
    }

    world
        .sample_at(position - IVec3::Y)
        .is_some_and(|(cell, fluid, _)| cell.is_none() && fluid.is_none())
}

fn horizontal_direction_bit(offset: IVec3) -> Option<u8> {
    HORIZONTAL_NEIGHBORS
        .into_iter()
        .position(|candidate| candidate == offset)
        .map(|index| 1_u8 << index)
}

pub(super) fn enqueue_remesh(position: IVec3, remesh_queue: &mut ChunkRemeshQueue) {
    let center = chunk_coord_from_world(position);
    remesh_queue.enqueue_fluid_priority(center);

    for offset in CARDINAL_NEIGHBORS {
        let neighbor = chunk_coord_from_world(position + offset);
        if neighbor != center {
            remesh_queue.enqueue_fluid_priority(neighbor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{cell::VoxelCell, chunk::VoxelChunk};

    fn world_with_floor() -> VoxelWorld {
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());

        for z in 0..16 {
            for x in 0..16 {
                world.set_block_at(
                    IVec3::new(x, 0, z),
                    Some(VoxelCell::new("stone", Default::default())),
                );
            }
        }

        world
    }

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
    fn vertical_fall_resets_horizontal_spread_distance() {
        let position = IVec3::new(4, 3, 4);
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());
        world.set_fluid_at(
            position + IVec3::Y,
            Some(FluidCell::spreading(0, 1, 7)),
        );

        assert_eq!(
            desired_fluid(
                &world,
                position,
                None,
                None,
                &FluidRegistry::default(),
            ),
            Some(FluidCell::spreading(0, MAX_FLUID_LEVEL, 0)),
        );
    }

    #[test]
    fn falling_dynamic_fluid_does_not_spread_sideways() {
        let position = IVec3::new(4, 10, 4);
        let source = FluidCell::source(0, MAX_FLUID_LEVEL);
        let mut exposed_world = VoxelWorld::default();
        exposed_world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());

        assert!(can_spread_horizontally_from(
            &exposed_world,
            position,
            source,
        ));
        assert!(!can_spread_horizontally_from(
            &exposed_world,
            position,
            FluidCell::spreading(0, MAX_FLUID_LEVEL, 0),
        ));

        let mut covered_world = VoxelWorld::default();
        covered_world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());
        covered_world.set_fluid_at(position + IVec3::Y, Some(source));

        assert!(!can_spread_horizontally_from(
            &covered_world,
            position,
            source,
        ));
    }

    #[test]
    fn horizontal_flow_prefers_nearest_reachable_drop() {
        let origin = IVec3::new(6, 1, 6);
        let east = origin + IVec3::X;
        let north = origin + IVec3::NEG_Z;
        let mut world = world_with_floor();

        world.set_block_at(IVec3::new(8, 0, 6), None);

        assert!(horizontal_spread_is_preferred(
            &world, origin, east, 0, 7,
        ));
        assert!(!horizontal_spread_is_preferred(
            &world, origin, north, 0, 7,
        ));
    }

    #[test]
    fn horizontal_flow_spreads_normally_when_no_drop_is_reachable() {
        let origin = IVec3::new(6, 1, 6);
        let world = world_with_floor();

        for offset in HORIZONTAL_NEIGHBORS {
            assert!(horizontal_spread_is_preferred(
                &world,
                origin,
                origin + offset,
                0,
                7,
            ));
        }
    }
}
