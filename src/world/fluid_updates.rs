mod frontier;
mod solver;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        lighting::PendingLightingUpdates, update_queue::VoxelUpdateQueue, world::VoxelWorld,
    },
};

use self::solver::{desired_fluid, enqueue_remesh};
use super::{
    chunk_remesh::ChunkRemeshQueue,
    game_rules::GameRules,
    tick::WorldTickClock,
};

const MAX_FLUID_UPDATES_PER_FRAME: usize = 512;
const MAX_FLUID_STEPS_PER_FRAME: usize = 4;

#[derive(Resource, Default)]
pub(crate) struct PendingFluidUpdates {
    queue: VoxelUpdateQueue,
    accumulated_steps: HashMap<FluidId, f32>,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors(position);
    }

    pub(crate) fn enqueue_loaded_fluid_frontier(&mut self, world: &VoxelWorld, coord: IVec3) {
        frontier::enqueue_loaded_fluid_frontier(self, world, coord);
    }

    fn enqueue(&mut self, position: IVec3) {
        self.queue.enqueue(position);
    }

    fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    fn ready_steps(
        &mut self,
        elapsed_ticks: u32,
        ticks_per_second: u32,
        fluids: &FluidRegistry,
    ) -> HashMap<FluidId, usize> {
        let mut ready = HashMap::new();

        if elapsed_ticks == 0 {
            return ready;
        }

        for (fluid_id, definition) in fluids.iter() {
            let accumulator = self.accumulated_steps.entry(fluid_id).or_default();
            if definition.spread_speed <= f32::EPSILON {
                *accumulator = 0.0;
                continue;
            }

            let steps_per_tick = definition.spread_speed / ticks_per_second as f32;
            *accumulator += elapsed_ticks as f32 * steps_per_tick;
            let elapsed_steps = accumulator.floor() as usize;
            let steps = elapsed_steps.min(MAX_FLUID_STEPS_PER_FRAME);

            if elapsed_steps > 0 {
                *accumulator -= elapsed_steps as f32;
            }
            if steps > 0 {
                ready.insert(fluid_id, steps);
            }
        }

        ready
    }

    fn clear(&mut self) {
        self.queue.clear();
        self.accumulated_steps.clear();
    }
}

pub(super) fn clear_fluid_updates(mut pending: ResMut<PendingFluidUpdates>) {
    pending.clear();
}

pub(super) fn process_fluid_updates(
    world_ticks: Res<WorldTickClock>,
    game_rules: Res<GameRules>,
    fluids: Res<FluidRegistry>,
    mut world: ResMut<VoxelWorld>,
    mut pending: ResMut<PendingFluidUpdates>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let ready_steps = pending.ready_steps(
        world_ticks.ticks_this_frame(),
        game_rules.ticks_per_second(),
        &fluids,
    );
    let max_steps = ready_steps.values().copied().max().unwrap_or(0);
    if max_steps == 0 {
        return;
    }

    let mut remaining_budget = MAX_FLUID_UPDATES_PER_FRAME;

    for step_index in 0..max_steps {
        if remaining_budget == 0 {
            break;
        }

        let batch_len = pending.queue.len().min(remaining_budget);
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
            pending.enqueue_voxel_edit(position);
        }
    }
}
