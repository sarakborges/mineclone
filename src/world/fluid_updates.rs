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

    let below = position - IVec3::Y;
    let supported = position.y == 0 || world.is_solid(below) || world.fluid_at(below).is_some();
    if !supported {
        return None;
    }

    let mut strongest: Option<(u8, FluidId)> = None;

    for offset in HORIZONTAL_NEIGHBORS {
        let Some(neighbor) = world.fluid_at(position + offset) else {
            continue;
        };
        let definition = fluids
            .get(neighbor.fluid_id)
            .unwrap_or_else(|| panic!("missing fluid definition for id {}", neighbor.fluid_id));
        let Some(level) = horizontal_spread_level(neighbor.level, definition.max_spread) else {
            continue;
        };

        let candidate = (level, neighbor.fluid_id);
        if strongest.is_none_or(|current| {
            candidate.0 > current.0 || (candidate.0 == current.0 && candidate.1 < current.1)
        }) {
            strongest = Some(candidate);
        }
    }

    strongest.map(|(level, fluid_id)| FluidCell::flowing(fluid_id, level))
}

fn horizontal_spread_level(neighbor_level: u8, max_spread: u8) -> Option<u8> {
    let distance = MAX_FLUID_LEVEL
        .saturating_sub(neighbor_level)
        .saturating_add(1);
    if distance > max_spread {
        return None;
    }

    let level = neighbor_level.saturating_sub(1);
    (level > 0).then_some(level)
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
    fn horizontal_spread_respects_data_driven_range() {
        assert_eq!(horizontal_spread_level(MAX_FLUID_LEVEL, 0), None);
        assert_eq!(horizontal_spread_level(MAX_FLUID_LEVEL, 1), Some(7));
        assert_eq!(horizontal_spread_level(7, 1), None);
        assert_eq!(horizontal_spread_level(7, 2), Some(6));
        assert_eq!(horizontal_spread_level(2, 7), Some(1));
        assert_eq!(horizontal_spread_level(1, 7), None);
    }
}
