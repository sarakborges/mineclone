use bevy::{
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        coordinates::chunk_coord_from_world,
        deduplicated_queue::DeduplicatedQueue,
        neighbors::HORIZONTAL_NEIGHBORS,
        world::VoxelWorld,
    },
};

use super::{
    frontier::visit_loaded_fluid_frontier_targets,
    solver::{FluidSolverScratch, desired_fluid_with_scratch},
};

/// Resolve generated fluid frontiers immediately inside the newly generated
/// chunk set. This is worldgen convergence, not gameplay simulation:
/// authored spread cadence is intentionally ignored so the first published
/// mesh already contains the settled state.
///
/// Targets outside `generated_chunks` are left to the runtime scheduler. That
/// keeps streaming generation from silently mutating previously persisted or
/// already-published neighboring chunks.
pub(crate) fn settle_generated_fluid_chunks(
    world: &mut VoxelWorld,
    fluids: &FluidRegistry,
    generated_chunks: &HashSet<IVec3>,
) {
    if generated_chunks.is_empty() {
        return;
    }

    let mut coords = generated_chunks.iter().copied().collect::<Vec<_>>();
    coords.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));

    let mut queue = DeduplicatedQueue::default();
    for coord in coords {
        visit_loaded_fluid_frontier_targets(
            world,
            coord,
            &mut |_fluid_id, target, priority| {
                if !generated_chunks.contains(&chunk_coord_from_world(target)) {
                    return;
                }
                if priority {
                    queue.enqueue_front(target);
                } else {
                    queue.enqueue(target);
                }
            },
        );
    }

    let mut scratch = FluidSolverScratch::default();
    while let Some(position) = queue.pop() {
        if !generated_chunks.contains(&chunk_coord_from_world(position)) {
            continue;
        }

        let Some((cell, current, _)) = world.sample_at(position) else {
            continue;
        };
        let desired = desired_fluid_with_scratch(
            world,
            position,
            cell,
            current,
            fluids,
            &mut scratch,
        );
        if current == desired {
            continue;
        }

        if world.set_derived_fluid_at(position, desired).is_none() {
            continue;
        }

        enqueue_changed_neighborhood(&mut queue, position, generated_chunks);
    }
}

fn enqueue_changed_neighborhood(
    queue: &mut DeduplicatedQueue<IVec3>,
    position: IVec3,
    generated_chunks: &HashSet<IVec3>,
) {
    for target in std::iter::once(position)
        .chain(std::iter::once(position - IVec3::Y))
        .chain(HORIZONTAL_NEIGHBORS.into_iter().map(|offset| position + offset))
    {
        if target.y >= 0 && generated_chunks.contains(&chunk_coord_from_world(target)) {
            queue.enqueue(target);
        }
    }
}
