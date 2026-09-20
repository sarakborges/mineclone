mod frontier;
mod solver;

use std::{
    collections::{BTreeMap, VecDeque},
    io,
    time::Duration,
};

use bevy::{
    ecs::system::SystemParam,
    platform::collections::HashMap,
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        coordinates::chunk_coord_from_world,
        deduplicated_queue::DeduplicatedQueue,
        fluid::FluidCell,
        lighting::PendingLightingUpdates,
        neighbors::HORIZONTAL_NEIGHBORS,
        update_queue::VoxelUpdateQueue,
        world::VoxelWorld,
    },
};

use self::solver::{
    FluidSolverMetrics, FluidSolverScratch, desired_fluid_with_scratch, enqueue_remesh,
};
use super::{
    chunk_remesh::ChunkRemeshQueue,
    game_rules::GameRules,
    tick::WorldTickClock,
    WorldLoadMode,
    work_budget::FrameWorkBudget,
};

const FLUID_UPDATE_BUDGET: Duration = Duration::from_millis(1);
const FLUID_CATCHUP_BUDGET: Duration = Duration::from_millis(3);
const MIN_FLUID_UPDATES_BEFORE_BUDGET_CHECK: usize = 64;
const MAX_FLUID_UPDATES_PER_FRAME: usize = 512;
const MAX_FLUID_CATCHUP_UPDATES_PER_FRAME: usize = 2_048;
const FLUID_CATCHUP_QUEUE_THRESHOLD: usize = 512;
const FLUID_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 10.0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FluidTickKey {
    fluid_id: FluidId,
    position: IVec3,
}

#[derive(Default)]
struct ChunkFairFluidQueue {
    by_chunk: HashMap<IVec3, DeduplicatedQueue<FluidTickKey>>,
    active_chunks: DeduplicatedQueue<IVec3>,
    len: usize,
}

impl ChunkFairFluidQueue {
    fn enqueue(&mut self, key: FluidTickKey) {
        let coord = chunk_coord_from_world(key.position);
        let queue = self.by_chunk.entry(coord).or_default();
        if !queue.enqueue(key) {
            return;
        }

        self.len += 1;
        self.active_chunks.enqueue(coord);
    }

    fn enqueue_front(&mut self, key: FluidTickKey) {
        let coord = chunk_coord_from_world(key.position);
        let queue = self.by_chunk.entry(coord).or_default();
        let already_queued = queue.contains(key);
        queue.enqueue_front(key);
        if !already_queued {
            self.len += 1;
        }

        // Priority is local to the chunk. Re-promoting an active chunk globally
        // would let a busy fluid section starve every neighboring section.
        self.active_chunks.enqueue(coord);
    }

    fn pop(&mut self) -> Option<FluidTickKey> {
        loop {
            let coord = self.active_chunks.pop()?;
            let (key, has_more) = {
                let queue = self
                    .by_chunk
                    .get_mut(&coord)
                    .expect("active fluid wake chunk must have a queue");
                let key = queue.pop();
                (key, queue.len() > 0)
            };

            if has_more {
                self.active_chunks.enqueue(coord);
            } else {
                self.by_chunk.remove(&coord);
            }

            let Some(key) = key else {
                continue;
            };
            self.len = self
                .len
                .checked_sub(1)
                .expect("fluid wake queue length cannot underflow");
            return Some(key);
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn values(&self) -> impl Iterator<Item = FluidTickKey> + '_ {
        self.by_chunk.values().flat_map(|queue| queue.values())
    }
}

#[derive(Default)]
struct ScheduledFluidBucket {
    by_chunk: HashMap<IVec3, VecDeque<FluidTickKey>>,
    active_chunks: VecDeque<IVec3>,
}

impl ScheduledFluidBucket {
    fn push(&mut self, key: FluidTickKey) {
        let coord = chunk_coord_from_world(key.position);
        let queue = self.by_chunk.entry(coord).or_default();
        if queue.is_empty() {
            self.active_chunks.push_back(coord);
        }
        queue.push_back(key);
    }

    fn pop(&mut self) -> Option<FluidTickKey> {
        loop {
            let coord = self.active_chunks.pop_front()?;
            let (key, has_more) = {
                let queue = self
                    .by_chunk
                    .get_mut(&coord)
                    .expect("active scheduled fluid chunk must have a queue");
                let key = queue.pop_front();
                (key, !queue.is_empty())
            };

            if has_more {
                self.active_chunks.push_back(coord);
            } else {
                self.by_chunk.remove(&coord);
            }

            if let Some(key) = key {
                return Some(key);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.active_chunks.is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct SavedFluidUpdates {
    #[serde(default)]
    topology: Vec<[i32; 3]>,
    #[serde(default)]
    wakes: Vec<SavedFluidWake>,
    #[serde(default)]
    scheduled: Vec<SavedScheduledFluidTick>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SavedFluidWake {
    fluid: String,
    position: [i32; 3],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SavedScheduledFluidTick {
    fluid: String,
    position: [i32; 3],
    remaining_ticks: u64,
}


#[derive(Resource, Default)]
pub(crate) struct PendingFluidUpdates {
    // Runtime block edits are topology wake-ups. They are resolved against the
    // current world state and converted into scheduled fluid ticks.
    topology_queue: VoxelUpdateQueue,
    // Generated/streamed fluid frontiers arrive here without a due time. The
    // runtime scheduler assigns the authored fluid delay on the next update.
    wake_queue: ChunkFairFluidQueue,
    // Minecraft-style scheduled ticks: one due world tick per (fluid, voxel).
    // Earlier reschedules replace later ones; stale bucket records are ignored.
    // Equal-due work is round-robin by chunk so frame budgets cannot turn
    // spatial iteration order into visible chunk-by-chunk propagation.
    scheduled: BTreeMap<u64, ScheduledFluidBucket>,
    scheduled_due: HashMap<FluidTickKey, u64>,
    // Due work whose chunk is currently not resident. It is reactivated when
    // streaming brings that chunk back instead of being silently discarded.
    dormant_scheduled: HashMap<IVec3, Vec<FluidTickKey>>,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.topology_queue
            .enqueue_with_neighbors_priority(position);
    }

    pub(crate) fn capture_saved(
        &self,
        current_tick: u64,
        fluids: &FluidRegistry,
    ) -> io::Result<SavedFluidUpdates> {
        let mut topology = self
            .topology_queue
            .values()
            .map(|position| position.to_array())
            .collect::<Vec<_>>();
        topology.sort_unstable();

        let mut wakes = self
            .wake_queue
            .values()
            .map(|key| {
                Ok(SavedFluidWake {
                    fluid: fluid_name(fluids, key.fluid_id)?.to_owned(),
                    position: key.position.to_array(),
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        wakes.sort_unstable_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then_with(|| left.fluid.cmp(&right.fluid))
        });

        let mut scheduled = self
            .scheduled_due
            .iter()
            .map(|(key, due_tick)| {
                Ok(SavedScheduledFluidTick {
                    fluid: fluid_name(fluids, key.fluid_id)?.to_owned(),
                    position: key.position.to_array(),
                    remaining_ticks: due_tick.saturating_sub(current_tick),
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        for keys in self.dormant_scheduled.values() {
            for key in keys {
                scheduled.push(SavedScheduledFluidTick {
                    fluid: fluid_name(fluids, key.fluid_id)?.to_owned(),
                    position: key.position.to_array(),
                    remaining_ticks: 0,
                });
            }
        }
        scheduled.sort_unstable_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then_with(|| left.fluid.cmp(&right.fluid))
                .then_with(|| left.remaining_ticks.cmp(&right.remaining_ticks))
        });
        scheduled.dedup_by(|left, right| {
            left.position == right.position && left.fluid == right.fluid
        });

        Ok(SavedFluidUpdates {
            topology,
            wakes,
            scheduled,
        })
    }

    pub(crate) fn from_saved(
        saved: &SavedFluidUpdates,
        fluids: &FluidRegistry,
    ) -> io::Result<Self> {
        let mut pending = Self::default();

        for position in &saved.topology {
            let position = IVec3::from_array(*position);
            validate_saved_position(position)?;
            pending.topology_queue.enqueue(position);
        }
        for wake in &saved.wakes {
            let position = IVec3::from_array(wake.position);
            validate_saved_position(position)?;
            let fluid_id = saved_fluid_id(fluids, &wake.fluid)?;
            pending.enqueue_fluid(fluid_id, position);
        }
        for tick in &saved.scheduled {
            let position = IVec3::from_array(tick.position);
            validate_saved_position(position)?;
            let fluid_id = saved_fluid_id(fluids, &tick.fluid)?;
            pending.schedule_at(
                FluidTickKey { fluid_id, position },
                tick.remaining_ticks,
            );
        }

        Ok(pending)
    }

    pub(crate) fn reactivate_loaded_chunk(&mut self, coord: IVec3, current_tick: u64) {
        let Some(keys) = self.dormant_scheduled.remove(&coord) else {
            return;
        };
        for key in keys {
            self.schedule_at(key, current_tick);
        }
    }

    fn defer_unloaded(&mut self, key: FluidTickKey) {
        let coord = chunk_coord_from_world(key.position);
        let entries = self.dormant_scheduled.entry(coord).or_default();
        if !entries.contains(&key) {
            entries.push(key);
        }
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
        self.scheduled.entry(due_tick).or_default().push(key);
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
                let key = bucket.pop();
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
    load_mode: Res<WorldLoadMode>,
    mut pending: ResMut<PendingFluidUpdates>,
) {
    if *load_mode == WorldLoadMode::New {
        *pending = PendingFluidUpdates::default();
    }

    let mut loaded = world.loaded_chunk_coords().collect::<Vec<_>>();
    loaded.sort_by_key(|coord| (coord.y, coord.z, coord.x));
    for coord in loaded {
        frontier::enqueue_resident_fluid_frontier(&mut pending, &world, coord);
    }
}

#[derive(Default)]
struct FluidPerformanceDiagnostics {
    timer: Option<Timer>,
    totals: FluidSolverMetrics,
    active_frames: u64,
}

impl FluidPerformanceDiagnostics {
    fn record(
        &mut self,
        delta: Duration,
        metrics: FluidSolverMetrics,
        pending: &PendingFluidUpdates,
        catch_up: bool,
    ) {
        if metrics.desired_evaluations > 0 {
            self.active_frames = self.active_frames.saturating_add(1);
        }
        self.totals.accumulate(metrics);

        let timer = self.timer.get_or_insert_with(|| {
            Timer::from_seconds(FLUID_DIAGNOSTIC_INTERVAL_SECONDS, TimerMode::Repeating)
        });
        timer.tick(delta);
        if !timer.just_finished() || self.totals.desired_evaluations == 0 {
            return;
        }

        let searches_per_desired =
            self.totals.downhill_searches as f64 / self.totals.desired_evaluations as f64;
        let nodes_per_search = if self.totals.downhill_searches == 0 {
            0.0
        } else {
            self.totals.downhill_nodes as f64 / self.totals.downhill_searches as f64
        };

        info!(
            "fluid solver diagnostics: desired={} horizontal_candidates={} downhill_searches={} downhill_nodes={} searches_per_desired={searches_per_desired:.3} nodes_per_search={nodes_per_search:.2} active_frames={} topology_backlog={} wake_backlog={} scheduled_backlog={} dormant_chunks={} catch_up={catch_up}",
            self.totals.desired_evaluations,
            self.totals.horizontal_candidates,
            self.totals.downhill_searches,
            self.totals.downhill_nodes,
            self.active_frames,
            pending.topology_queue.len(),
            pending.wake_queue.len(),
            pending.scheduled_due.len(),
            pending.dormant_scheduled.len(),
        );

        self.totals = FluidSolverMetrics::default();
        self.active_frames = 0;
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
    time: Res<Time<Real>>,
    mut solver_scratch: Local<FluidSolverScratch>,
    mut diagnostics: Local<FluidPerformanceDiagnostics>,
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

    diagnostics.record(
        time.delta(),
        solver_scratch.take_metrics(),
        &runtime.pending,
        catch_up,
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
            runtime.pending.defer_unloaded(scheduled);
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

fn fluid_name(fluids: &FluidRegistry, fluid_id: FluidId) -> io::Result<&str> {
    fluids
        .get(fluid_id)
        .map(|definition| definition.id.as_str())
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::InvalidData,
            format!("missing fluid definition for saved runtime id {fluid_id}"),
        ))
}

fn saved_fluid_id(fluids: &FluidRegistry, fluid: &str) -> io::Result<FluidId> {
    fluids.id_of(fluid).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown saved fluid tick definition: {fluid}"),
        )
    })
}

fn validate_saved_position(position: IVec3) -> io::Result<()> {
    if position.y < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "saved fluid update position cannot be below world floor",
        ));
    }
    Ok(())
}

fn fluid_tick_delay_for_id(
    fluids: &FluidRegistry,
    fluid_id: FluidId,
    ticks_per_second: u32,
) -> Option<u64> {
    let definition = fluids
        .get(fluid_id)
        .unwrap_or_else(|| panic!("missing fluid definition for id {fluid_id}"));
    fluid_tick_delay_ticks(definition.spread_speed, ticks_per_second)
}

fn fluid_tick_delay_ticks(spread_speed: f32, ticks_per_second: u32) -> Option<u64> {
    if spread_speed <= f32::EPSILON || ticks_per_second == 0 {
        return None;
    }

    let ticks = (ticks_per_second as f64 / f64::from(spread_speed))
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

    #[test]
    fn wake_queue_round_robins_chunks_and_keeps_local_priority() {
        let mut queue = ChunkFairFluidQueue::default();
        let a1 = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(1, 2, 3),
        };
        let a2 = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(2, 2, 3),
        };
        let b1 = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(17, 2, 3),
        };

        queue.enqueue(a1);
        queue.enqueue(a2);
        queue.enqueue(b1);
        queue.enqueue_front(a2);

        assert_eq!(queue.pop(), Some(a2));
        assert_eq!(queue.pop(), Some(b1));
        assert_eq!(queue.pop(), Some(a1));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn equal_due_fluid_ticks_round_robin_between_chunks() {
        let mut pending = PendingFluidUpdates::default();
        let a1 = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(1, 2, 3),
        };
        let a2 = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(2, 2, 3),
        };
        let b1 = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(17, 2, 3),
        };
        let b2 = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(18, 2, 3),
        };

        for key in [a1, a2, b1, b2] {
            pending.schedule_at(key, 10);
        }

        assert_eq!(pending.pop_due(10), Some((a1, 10)));
        assert_eq!(pending.pop_due(10), Some((b1, 10)));
        assert_eq!(pending.pop_due(10), Some((a2, 10)));
        assert_eq!(pending.pop_due(10), Some((b2, 10)));
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
        assert_eq!(fluid_tick_delay_ticks(16.0, 40), Some(3));
        assert_eq!(fluid_tick_delay_ticks(4.0, 40), Some(10));
        assert_eq!(fluid_tick_delay_ticks(80.0, 40), Some(1));
        assert_eq!(fluid_tick_delay_ticks(0.0, 40), None);
    }

    #[test]
    fn unloaded_due_tick_is_reactivated_by_chunk_residency() {
        let mut pending = PendingFluidUpdates::default();
        let key = FluidTickKey {
            fluid_id: 0,
            position: IVec3::new(1, 2, 3),
        };
        pending.defer_unloaded(key);

        assert_eq!(pending.pop_due(9), None);

        pending.reactivate_loaded_chunk(IVec3::ZERO, 9);
        assert_eq!(pending.pop_due(9), Some((key, 9)));
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
