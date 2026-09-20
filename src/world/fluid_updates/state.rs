use std::{
    collections::{BTreeMap, VecDeque},
    io,
};

use bevy::{
    platform::collections::HashMap,
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    content::fluid::{FluidId, FluidRegistry},
    voxel::{
        coordinates::chunk_coord_from_world,
        deduplicated_queue::DeduplicatedQueue,
        neighbors::HORIZONTAL_NEIGHBORS,
        update_queue::VoxelUpdateQueue,
        world::VoxelWorld,
    },
};

use super::frontier;

const FLUID_CATCHUP_QUEUE_THRESHOLD: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingFluidBacklog {
    pub(super) topology: usize,
    pub(super) wakes: usize,
    pub(super) scheduled: usize,
    pub(super) dormant_chunks: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct FluidTickKey {
    pub(super) fluid_id: FluidId,
    pub(super) position: IVec3,
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

    pub(super) fn defer_unloaded(&mut self, key: FluidTickKey) {
        let coord = chunk_coord_from_world(key.position);
        let entries = self.dormant_scheduled.entry(coord).or_default();
        if !entries.contains(&key) {
            entries.push(key);
        }
    }

    pub(crate) fn enqueue_loaded_fluid_frontier(&mut self, world: &VoxelWorld, coord: IVec3) {
        frontier::enqueue_loaded_fluid_frontier(self, world, coord);
    }

    pub(super) fn enqueue_fluid(&mut self, fluid_id: FluidId, position: IVec3) {
        if position.y < 0 {
            return;
        }
        self.wake_queue.enqueue(FluidTickKey { fluid_id, position });
    }

    pub(super) fn enqueue_fluid_priority(&mut self, fluid_id: FluidId, position: IVec3) {
        if position.y < 0 {
            return;
        }
        self.wake_queue
            .enqueue_front(FluidTickKey { fluid_id, position });
    }

    pub(super) fn pop_wake(&mut self) -> Option<FluidTickKey> {
        self.wake_queue.pop()
    }

    pub(super) fn pop_topology(&mut self) -> Option<IVec3> {
        self.topology_queue.pop()
    }

    pub(super) fn schedule_at(&mut self, key: FluidTickKey, due_tick: u64) {
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

    pub(super) fn pop_due(&mut self, current_tick: u64) -> Option<(FluidTickKey, u64)> {
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

    pub(super) fn schedule_neighborhood(&mut self, fluid_id: FluidId, position: IVec3, due_tick: u64) {
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

    pub(super) fn should_catch_up(&self) -> bool {
        self.topology_queue
            .len()
            .saturating_add(self.wake_queue.len())
            .saturating_add(self.scheduled_due.len())
            >= FLUID_CATCHUP_QUEUE_THRESHOLD
    }

    pub(super) fn topology_len(&self) -> usize {
        self.topology_queue.len()
    }

    pub(super) fn backlog(&self) -> PendingFluidBacklog {
        PendingFluidBacklog {
            topology: self.topology_queue.len(),
            wakes: self.wake_queue.len(),
            scheduled: self.scheduled_due.len(),
            dormant_chunks: self.dormant_scheduled.len(),
        }
    }
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_queue_round_robins_chunks_and_keeps_local_priority() {
        let mut queue = ChunkFairFluidQueue::default();
        let a1 = FluidTickKey { fluid_id: 0, position: IVec3::new(1, 2, 3) };
        let a2 = FluidTickKey { fluid_id: 0, position: IVec3::new(2, 2, 3) };
        let b1 = FluidTickKey { fluid_id: 0, position: IVec3::new(17, 2, 3) };

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
        let a1 = FluidTickKey { fluid_id: 0, position: IVec3::new(1, 2, 3) };
        let a2 = FluidTickKey { fluid_id: 0, position: IVec3::new(2, 2, 3) };
        let b1 = FluidTickKey { fluid_id: 0, position: IVec3::new(17, 2, 3) };
        let b2 = FluidTickKey { fluid_id: 0, position: IVec3::new(18, 2, 3) };

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
        let key = FluidTickKey { fluid_id: 0, position: IVec3::new(4, 5, 6) };

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
        let key = FluidTickKey { fluid_id: 0, position: IVec3::new(2, 3, 4) };
        pending.schedule_at(key, 25);

        assert_eq!(pending.pop_due(24), None);
        assert_eq!(pending.pop_due(25), Some((key, 25)));
    }

    #[test]
    fn unloaded_due_tick_is_reactivated_by_chunk_residency() {
        let mut pending = PendingFluidUpdates::default();
        let key = FluidTickKey { fluid_id: 0, position: IVec3::new(1, 2, 3) };
        pending.defer_unloaded(key);

        assert_eq!(pending.pop_due(9), None);

        pending.reactivate_loaded_chunk(IVec3::ZERO, 9);
        assert_eq!(pending.pop_due(9), Some((key, 9)));
    }
}
