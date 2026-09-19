mod frontier;
mod solver;

use std::{
    collections::{BTreeMap, VecDeque},
    time::Duration,
};

use bevy::{
    ecs::system::SystemParam,
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    content::fluid::{FluidDefinition, FluidId, FluidRegistry},
    voxel::{
        deduplicated_queue::DeduplicatedQueue,
        fluid::FluidCell,
        lighting::PendingLightingUpdates,
        neighbors::HORIZONTAL_NEIGHBORS,
        update_queue::VoxelUpdateQueue,
        world::VoxelWorld,
    },
};

use self::solver::{FluidSolverScratch, desired_fluid_with_scratch, enqueue_remesh};
use super::{
    chunk_remesh::ChunkRemeshQueue,
    game_rules::GameRules,
    tick::WorldTickClock,
    work_budget::FrameWorkBudget,
};

const FLUID_UPDATE_BUDGET: Duration = Duration::from_millis(1);
const FLUID_CATCHUP_BUDGET: Duration = Duration::from_millis(3);
const MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK: usize = 64;
const MAX_FLUID_UPDATES_PER_FRAME: usize = 512;
const MAX_FLUID_CATCHUP_UPDATES_PER_FRAME: usize = 2_048;
const FLUID_CATCHUP_QUEUE_THRESHOLD: usize = 512;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FluidTickKey {
    fluid_id: FluidId,
    position: IVec3,
}

#[derive(Resource, Default)]
pub(crate) struct PendingFluidUpdates {
    // Runtime block edits are topology wake-ups. They are resolved against the
    // current world state and converted into scheduled fluid ticks.
    topology_queue: VoxelUpdateQueue,
    // Generated/streamed fluid frontiers arrive here without a due time. The
    // runtime scheduler assigns the authored fluid delay on the next update.
    wake_queue: DeduplicatedQueue<FluidTickKey>,
    // Minecraft-style scheduled ticks: one due world tick per (fluid, voxel).
    // Earlier reschedules replace later ones; stale bucket records are ignored.
    scheduled: BTreeMap<u64, VecDeque<FluidTickKey>>,
    scheduled_due: HashMap<FluidTickKey, u64>,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.topology_queue
            .enqueue_with_neighbors_priority(position);
    }

    pub(crate) fn enqueue_loaded_fluid_frontier(&mut self, world: &VoxelWorld, coord: IVec3) {
        frontier::enqueue_loaded_fluid_frontier(self, world, coord);
    }

    fn enqueue_fluid(&mut self, fluid_id: FluidId, position: IVec3) {
        if position.y < 0 {
            return;
        }
        self.wake_queue.enqueue(FluidTickKey { fluid_id, position });
    }

    fn enqueue_fluid_priority(&mut self, fluid_id: FluidId, position: IVec3) {
        if position.y < 0 {
            return;
        }
        self.wake_queue
            .enqueue_front(FluidTickKey { fluid_id, position });
    }

    fn pop_wake(&mut self) -> Option<FluidTickKey> {
        self.wake_queue.pop()
    }

    fn pop_topology(&mut self) -> Option<IVec3> {
        self.topology_queue.pop()
    }

    fn schedule_at(&mut self, key: FluidTickKey, due_tick: u64) {
        if key.position.y < 0 {
            return;
        }

        if self
            .scheduled_due
            .get(&key)
            .is_some_and(|existing| *existing <= due_tick)
        {
            return;
        }

        self.scheduled_due.insert(key, due_tick);
        self.scheduled.entry(due_tick).or_default().push_back(key);
    }

    fn pop_due(&mut self, current_tick: u64) -> Option<(FluidTickKey, u64)> {
        loop {
            let due_tick = *self.scheduled.first_key_value()?.0;
            if due_tick > current_tick {
                return None;
            }

            let (key, bucket_empty) = {
                let bucket = self
                    .scheduled
                    .get_mut(&due_tick)
                    .expect("first scheduled fluid bucket must exist");
                let key = bucket.pop_front();
                (key, bucket.is_empty())
            };
            if bucket_empty {
                self.scheduled.remove(&due_tick);
            }

            let Some(key) = key else {
                continue;
            };
            if self.scheduled_due.get(&key).copied() != Some(due_tick) {
                continue;
            }

            self.scheduled_due.remove(&key);
            return Some((key, due_tick));
        }
    }

    fn schedule_neighborhood(&mut self, fluid_id: FluidId, position: IVec3, due_tick: u64) {
        self.schedule_at(FluidTickKey { fluid_id, position }, due_tick);
        self.schedule_at(
            FluidTickKey {
                fluid_id,
                position: position - IVec3::Y,
            },
            due_tick,
        );
        for offset in HORIZONTAL_NEIGHBORS {
            self.schedule_at(
                FluidTickKey {
                    fluid_id,
                    position: position + offset,
                },
                due_tick,
            );
        }
    }

    fn should_catch_up(&self) -> bool {
        self.topology_queue
            .len()
            .saturating_add(self.wake_queue.len())
            .saturating_add(self.scheduled_due.len())
            >= FLUID_CATCHUP_QUEUE_THRESHOLD
    }
}

pub(super) fn reseed_loaded_fluid_frontiers(
    world: Res<VoxelWorld>,
    mut pending: ResMut<PendingFluidUpdates>,
) {
    *pending = PendingFluidUpdates::default();

    let mut loaded = world.loaded_chunk_coords().collect::<Vec<_>>();
    loaded.sort_by_key(|coord| (coord.y, coord.z, coord.x));
    for coord in loaded {
        frontier::enqueue_resident_fluid_frontier(&mut pending, &world, coord);
    }
}

#[derive(SystemParam)]
pub(super) struct FluidSimulationRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    pending: ResMut<'w, PendingFluidUpdates>,
    lighting: ResMut<'w, PendingLightingUpdates>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub(super) fn process_fluid_updates(
    world_ticks: Res<WorldTickClock>,
    game_rules: Res<GameRules>,
    fluids: Res<FluidRegistry>,
    mut solver_scratch: Local<FluidSolverScratch>,
    mut runtime: FluidSimulationRuntime,
) {
    let current_tick = world_ticks.current_tick();
    let ticks_per_second = game_rules.ticks_per_second();

    let catch_up = runtime.pending.should_catch_up();
    let mut budget = if catch_up {
        FrameWorkBudget::new(
            FLUID_CATCHUP_BUDGET,
            MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK,
        )
        .with_maximum_items(MAX_FLUID_CATCHUP_UPDATES_PER_FRAME)
    } else {
        FrameWorkBudget::new(
            FLUID_UPDATE_BUDGET,
            MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK,
        )
        .with_maximum_items(MAX_FLUID_UPDATES_PER_FRAME)
    };

    classify_topology_updates(
        &mut runtime,
        &fluids,
        &mut solver_scratch,
        current_tick,
        ticks_per_second,
        &mut budget,
    );

    process_due_fluid_ticks(
        &mut runtime,
        &fluids,
        &mut solver_scratch,
        current_tick,
        ticks_per_second,
        &mut budget,
    );

    schedule_frontier_wakes(
        &mut runtime.pending,
        &fluids,
        current_tick,
        ticks_per_second,
        &mut budget,
    );
}

fn classify_topology_updates(
    runtime: &mut FluidSimulationRuntime<'_>,
    fluids: &FluidRegistry,
    solver_scratch: &mut FluidSolverScratch,
    current_tick: u64,
    ticks_per_second: u32,
    budget: &mut FrameWorkBudget,
) {
    let batch_len = runtime.pending.topology_queue.len();
    for _ in 0..batch_len {
        if budget.exhausted() {
            break;
        }

        let Some(position) = runtime.pending.pop_topology() else {
            break;
        };
        budget.record(1);

        let Some((cell, current, _)) = runtime.world.sample_at(position) else {
            continue;
        };
        let desired = desired_fluid_with_scratch(
            &runtime.world,
            position,
            cell,
            current,
            fluids,
            solver_scratch,
        );
        if current == desired {
            continue;
        }

        let Some(fluid_id) = transition_fluid_id(current, desired) else {
            continue;
        };
        schedule_fluid_tick_after_delay(
            &mut runtime.pending,
            fluids,
            fluid_id,
            position,
            current_tick,
            ticks_per_second,
        );
    }
}

fn process_due_fluid_ticks(
    runtime: &mut FluidSimulationRuntime<'_>,
    fluids: &FluidRegistry,
    solver_scratch: &mut FluidSolverScratch,
    current_tick: u64,
    ticks_per_second: u32,
    budget: &mut FrameWorkBudget,
) {
    while !budget.exhausted() {
        let Some((scheduled, _due_tick)) = runtime.pending.pop_due(current_tick) else {
            break;
        };
        budget.record(1);

        let position = scheduled.position;
        let Some((cell, current, _)) = runtime.world.sample_at(position) else {
            continue;
        };
        let desired = desired_fluid_with_scratch(
            &runtime.world,
            position,
            cell,
            current,
            fluids,
            solver_scratch,
        );
        if current == desired {
            continue;
        }

        let Some(transition_fluid_id) = transition_fluid_id(current, desired) else {
            continue;
        };

        if transition_fluid_id != scheduled.fluid_id {
            schedule_fluid_tick_after_delay(
                &mut runtime.pending,
                fluids,
                transition_fluid_id,
                position,
                current_tick,
                ticks_per_second,
            );
            continue;
        }

        if runtime.world.set_fluid_at(position, desired).is_none() {
            continue;
        }

        runtime.lighting.enqueue_medium_edit(position);
        enqueue_remesh(position, &mut runtime.remesh_queue);
        schedule_changed_fluid_neighborhood(
            &mut runtime.pending,
            fluids,
            position,
            current,
            desired,
            current_tick,
            ticks_per_second,
        );
    }
}

fn schedule_frontier_wakes(
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    current_tick: u64,
    ticks_per_second: u32,
    budget: &mut FrameWorkBudget,
) {
    while !budget.exhausted() {
        let Some(wake) = pending.pop_wake() else {
            break;
        };
        budget.record(1);
        schedule_fluid_tick_after_delay(
            pending,
            fluids,
            wake.fluid_id,
            wake.position,
            current_tick,
            ticks_per_second,
        );
    }
}

fn schedule_changed_fluid_neighborhood(
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    position: IVec3,
    current: Option<FluidCell>,
    desired: Option<FluidCell>,
    current_tick: u64,
    ticks_per_second: u32,
) {
    let current_id = current.map(|fluid| fluid.fluid_id);
    let desired_id = desired.map(|fluid| fluid.fluid_id);

    if let Some(fluid_id) = current_id {
        schedule_fluid_neighborhood_after_delay(
            pending,
            fluids,
            fluid_id,
            position,
            current_tick,
            ticks_per_second,
        );
    }
    if let Some(fluid_id) = desired_id
        && Some(fluid_id) != current_id
    {
        schedule_fluid_neighborhood_after_delay(
            pending,
            fluids,
            fluid_id,
            position,
            current_tick,
            ticks_per_second,
        );
    }
}

fn schedule_fluid_neighborhood_after_delay(
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    fluid_id: FluidId,
    position: IVec3,
    current_tick: u64,
    ticks_per_second: u32,
) {
    let Some(delay) = fluid_tick_delay_for_id(fluids, fluid_id, ticks_per_second) else {
        return;
    };
    pending.schedule_neighborhood(fluid_id, position, current_tick.saturating_add(delay));
}

fn schedule_fluid_tick_after_delay(
    pending: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    fluid_id: FluidId,
    position: IVec3,
    current_tick: u64,
    ticks_per_second: u32,
) {
    let Some(delay) = fluid_tick_delay_for_id(fluids, fluid_id, ticks_per_second) else {
        return;
    };
    pending.schedule_at(
        FluidTickKey { fluid_id, position },
        current_tick.saturating_add(delay),
    );
}

fn fluid_tick_delay_for_id(
    fluids: &FluidRegistry,
    fluid_id: FluidId,
    ticks_per_second: u32,
) -> Option<u64> {
    let definition = fluids
        .get(fluid_id)
        .unwrap_or_else(|| panic!("missing fluid definition for id {fluid_id}"));
    fluid_tick_delay_ticks(definition, ticks_per_second)
}

fn fluid_tick_delay_ticks(
    definition: &FluidDefinition,
    ticks_per_second: u32,
) -> Option<u64> {
    if definition.spread_speed <= f32::EPSILON || ticks_per_second == 0 {
        return None;
    }

    let ticks = (ticks_per_second as f64 / f64::from(definition.spread_speed))
        .round()
        .max(1.0);
    Some(ticks as u64)
}

fn transition_fluid_id(
    current: Option<FluidCell>,
    desired: Option<FluidCell>,
) -> Option<FluidId> {
    desired
        .map(|fluid| fluid.fluid_id)
        .or_else(|| current.map(|fluid| fluid.fluid_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_definition(spread_speed: f32) -> FluidDefinition {
        FluidDefinition {
            id: "asteria:test".to_owned(),
            color: Default::default(),
            opacity: 1.0,
            roughness: 0.0,
            metallic: 0.0,
            light_dampening: 0,
            light_emission: 0,
            spread_speed,
            max_spread: 7,
        }
    }

    #[test]
    fn scheduled_fluid_tick_keeps_the_earliest_due_time() {
        let mut pending = PendingFluidUpdates::default();
        let key = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(4, 5, 6),
        };

        pending.schedule_at(key, 10);
        pending.schedule_at(key, 12);
        pending.schedule_at(key, 8);

        assert_eq!(pending.pop_due(7), None);
        assert_eq!(pending.pop_due(8), Some((key, 8)));
        assert_eq!(pending.pop_due(20), None);
    }

    #[test]
    fn future_fluid_tick_is_never_popped_early() {
        let mut pending = PendingFluidUpdates::default();
        let key = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(2, 3, 4),
        };
        pending.schedule_at(key, 25);

        assert_eq!(pending.pop_due(24), None);
        assert_eq!(pending.pop_due(25), Some((key, 25)));
    }

    #[test]
    fn spread_speed_quantizes_to_world_tick_delay() {
        assert_eq!(fluid_tick_delay_ticks(&test_definition(16.0), 40), Some(3));
        assert_eq!(fluid_tick_delay_ticks(&test_definition(4.0), 40), Some(10));
        assert_eq!(fluid_tick_delay_ticks(&test_definition(80.0), 40), Some(1));
        assert_eq!(fluid_tick_delay_ticks(&test_definition(0.0), 40), None);
    }

    #[test]
    fn transition_uses_destination_fluid_for_replacement() {
        let water = FluidCell::spreading(0, 4, 1);
        let lava = FluidCell::spreading(1, 8, 0);

        assert_eq!(transition_fluid_id(Some(water), Some(lava)), Some(1));
        assert_eq!(transition_fluid_id(Some(water), None), Some(0));
        assert_eq!(transition_fluid_id(None, Some(lava)), Some(1));
    }
}
