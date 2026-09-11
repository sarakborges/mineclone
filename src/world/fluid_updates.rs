use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        chunk::CHUNK_SIZE,
        fluid::{FluidCell, MAX_FLUID_LEVEL},
        lighting::PendingLightingUpdates,
        neighbors::CARDINAL_NEIGHBORS,
        world::VoxelWorld,
    },
};

use super::chunk_remesh::ChunkRemeshQueue;

const MAX_FLUID_UPDATES_PER_FRAME: usize = 512;
const MAX_FLUID_STEPS_PER_FRAME: usize = 4;
const HORIZONTAL_NEIGHBORS: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
const FLUID_SPREAD_TARGETS: [IVec3; 5] = [
    IVec3::NEG_Y,
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
];

#[derive(Resource, Default)]
pub(crate) struct PendingFluidUpdates {
    pending: VecDeque<IVec3>,
    queued: HashSet<IVec3>,
    accumulated_seconds: HashMap<FluidId, f32>,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.enqueue(position);
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue(position + offset);
        }
    }

    pub(crate) fn enqueue_loaded_fluid_frontier(&mut self, world: &VoxelWorld, coord: IVec3) {
        self.enqueue_chunk_spread_targets(world, coord);

        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_neighbor_boundary_spread_targets(world, coord + offset, -offset);
        }
    }

    fn enqueue_chunk_spread_targets(&mut self, world: &VoxelWorld, coord: IVec3) {
        if coord.y < 0 {
            return;
        }
        let Some(chunk) = world.chunk(coord) else {
            return;
        };

        let origin = coord * CHUNK_SIZE as i32;

        for local_y in 0..CHUNK_SIZE {
            for local_z in 0..CHUNK_SIZE {
                for local_x in 0..CHUNK_SIZE {
                    if chunk
                        .fluid_at(local_x as i32, local_y as i32, local_z as i32)
                        .is_none()
                    {
                        continue;
                    }

                    let position = origin
                        + IVec3::new(local_x as i32, local_y as i32, local_z as i32);
                    self.enqueue_spread_targets_from_fluid(world, position);
                }
            }
        }
    }

    fn enqueue_neighbor_boundary_spread_targets(
        &mut self,
        world: &VoxelWorld,
        coord: IVec3,
        direction: IVec3,
    ) {
        if coord.y < 0 {
            return;
        }
        let Some(chunk) = world.chunk(coord) else {
            return;
        };

        let size = CHUNK_SIZE as i32;
        let origin = coord * size;

        if direction.x != 0 {
            let local_x = if direction.x < 0 { 0 } else { size - 1 };
            for local_y in 0..size {
                for local_z in 0..size {
                    if chunk.fluid_at(local_x, local_y, local_z).is_none() {
                        continue;
                    }
                    self.enqueue_spread_targets_from_fluid(
                        world,
                        origin + IVec3::new(local_x, local_y, local_z),
                    );
                }
            }
            return;
        }

        if direction.y != 0 {
            let local_y = if direction.y < 0 { 0 } else { size - 1 };
            for local_z in 0..size {
                for local_x in 0..size {
                    if chunk.fluid_at(local_x, local_y, local_z).is_none() {
                        continue;
                    }
                    self.enqueue_spread_targets_from_fluid(
                        world,
                        origin + IVec3::new(local_x, local_y, local_z),
                    );
                }
            }
            return;
        }

        let local_z = if direction.z < 0 { 0 } else { size - 1 };
        for local_y in 0..size {
            for local_x in 0..size {
                if chunk.fluid_at(local_x, local_y, local_z).is_none() {
                    continue;
                }
                self.enqueue_spread_targets_from_fluid(
                    world,
                    origin + IVec3::new(local_x, local_y, local_z),
                );
            }
        }
    }

    fn enqueue_spread_targets_from_fluid(&mut self, world: &VoxelWorld, position: IVec3) {
        for offset in FLUID_SPREAD_TARGETS {
            let target = position + offset;
            if !world.is_loaded_at(target)
                || world.is_solid(target)
                || world.fluid_at(target).is_some()
            {
                continue;
            }

            self.enqueue(target);
        }
    }

    fn enqueue(&mut self, position: IVec3) {
        if position.y >= 0 && self.queued.insert(position) {
            self.pending.push_back(position);
        }
    }

    fn pop(&mut self) -> Option<IVec3> {
        let position = self.pending.pop_front()?;
        self.queued.remove(&position);
        Some(position)
    }

    fn ready_steps(&mut self, delta_seconds: f32, fluids: &FluidRegistry) -> HashMap<FluidId, usize> {
        let mut ready = HashMap::new();

        for (fluid_id, definition) in fluids.iter() {
            let accumulator = self.accumulated_seconds.entry(fluid_id).or_default();
            if definition.spread_speed <= f32::EPSILON {
                *accumulator = 0.0;
                continue;
            }

            let interval = 1.0 / definition.spread_speed;
            *accumulator += delta_seconds;
            let elapsed_steps = (*accumulator / interval).floor() as usize;
            let steps = elapsed_steps.min(MAX_FLUID_STEPS_PER_FRAME);

            if elapsed_steps > 0 {
                *accumulator -= elapsed_steps as f32 * interval;
            }
            if steps > 0 {
                ready.insert(fluid_id, steps);
            }
        }

        ready
    }

    fn clear(&mut self) {
        self.pending.clear();
        self.queued.clear();
        self.accumulated_seconds.clear();
    }
}

pub(super) fn clear_fluid_updates(mut pending: ResMut<PendingFluidUpdates>) {
    pending.clear();
}

pub(super) fn process_fluid_updates(
    time: Res<Time>,
    fluids: Res<FluidRegistry>,
    mut world: ResMut<VoxelWorld>,
    mut pending: ResMut<PendingFluidUpdates>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let ready_steps = pending.ready_steps(time.delta().as_secs_f32(), &fluids);
    let max_steps = ready_steps.values().copied().max().unwrap_or(0);
    if max_steps == 0 {
        return;
    }

    let mut remaining_budget = MAX_FLUID_UPDATES_PER_FRAME;

    for step_index in 0..max_steps {
        if remaining_budget == 0 {
            break;
        }

        let batch_len = pending.pending.len().min(remaining_budget);
        for _ in 0..batch_len {
            remaining_budget -= 1;

            let Some(position) = pending.pop() else {
                break;
            };
            if !world.is_loaded_at(position) {
                continue;
            }

            let current = world.fluid_at(position);
            let desired = desired_fluid(&world, position, current, &fluids);
            let Some(fluid_id) = current.or(desired).map(|fluid| fluid.fluid_id) else {
                continue;
            };
            let definition = fluids
                .get(fluid_id)
                .unwrap_or_else(|| panic!("missing fluid definition for id {fluid_id}"));
            let fluid_ready_steps = ready_steps.get(&fluid_id).copied().unwrap_or(0);

            if fluid_ready_steps <= step_index {
                if definition.spread_speed > f32::EPSILON {
                    pending.enqueue(position);
                }
                continue;
            }
            if current == desired {
                continue;
            }

            if world.set_fluid_at(position, desired).is_none() {
                continue;
            }

            lighting.enqueue_voxel_edit(position);
            enqueue_remesh(position, &mut remesh_queue);

            pending.enqueue(position);
            for offset in CARDINAL_NEIGHBORS {
                pending.enqueue(position + offset);
            }
        }
    }
}

fn desired_fluid(
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
    if fluid.is_source() || position.y == 0 {
        return true;
    }

    world.is_solid(position - IVec3::Y)
}

fn horizontal_spread_state(neighbor: FluidCell, max_spread: u16) -> Option<(u8, u16)> {
    let spread_distance = neighbor.spread_distance().saturating_add(1);
    if spread_distance > max_spread {
        return None;
    }

    let level = neighbor.level.saturating_sub(1).max(1);
    Some((level, spread_distance))
}

fn enqueue_remesh(position: IVec3, remesh_queue: &mut ChunkRemeshQueue) {
    let center = chunk_coord(position);
    remesh_queue.enqueue_priority(center);

    for offset in CARDINAL_NEIGHBORS {
        let neighbor = chunk_coord(position + offset);
        if neighbor != center {
            remesh_queue.enqueue(neighbor);
        }
    }
}

fn chunk_coord(position: IVec3) -> IVec3 {
    let size = CHUNK_SIZE as i32;
    IVec3::new(
        position.x.div_euclid(size),
        position.y.div_euclid(size),
        position.z.div_euclid(size),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn source_water_can_spill_over_an_edge_without_airborne_flow_fanning_out() {
        let world = VoxelWorld::default();
        let position = IVec3::new(4, 10, 4);

        assert!(can_spill_over_edge(
            &world,
            position,
            FluidCell::source(0, MAX_FLUID_LEVEL),
        ));
        assert!(!can_spill_over_edge(
            &world,
            position,
            FluidCell::flowing(0, MAX_FLUID_LEVEL),
        ));
    }
}
