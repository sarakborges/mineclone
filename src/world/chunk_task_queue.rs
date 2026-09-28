use std::collections::HashMap;

use bevy::{
    prelude::IVec3,
    tasks::{Task, futures::check_ready},
};

/// Identifies the immutable input snapshot used by an asynchronous chunk task.
///
/// Keeping this distinct from arbitrary counters prevents task publication code
/// from accidentally comparing a completed job against an unrelated world,
/// content, or presentation revision. Callers may continue supplying their
/// existing `u64` snapshot counters while migration is in progress; the task
/// boundary converts them immediately into this domain type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TaskInputRevision(u64);

impl From<u64> for TaskInputRevision {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl PartialEq<u64> for TaskInputRevision {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

pub(crate) struct CompletedChunkTask<T> {
    pub(crate) coord: IVec3,
    pub(crate) revision: TaskInputRevision,
    pub(crate) output: T,
}

struct PendingChunkTask<T> {
    revision: TaskInputRevision,
    task: Task<T>,
}

pub(crate) struct ChunkTaskQueue<T> {
    pending: HashMap<IVec3, PendingChunkTask<T>>,
}

impl<T> Default for ChunkTaskQueue<T> {
    fn default() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }
}

impl<T> ChunkTaskQueue<T> {
    pub(crate) fn len(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn contains(&self, coord: IVec3) -> bool {
        self.pending.contains_key(&coord)
    }

    pub(crate) fn cancel(&mut self, coord: IVec3) -> bool {
        self.pending.remove(&coord).is_some()
    }

    pub(crate) fn cancel_where(
        &mut self,
        mut predicate: impl FnMut(IVec3) -> bool,
    ) -> Vec<IVec3> {
        let coords = self
            .pending
            .keys()
            .copied()
            .filter(|coord| predicate(*coord))
            .collect::<Vec<_>>();
        for coord in &coords {
            self.pending.remove(coord);
        }
        coords
    }

    pub(crate) fn best_coord_by_key<K: Ord>(
        &self,
        mut key: impl FnMut(IVec3) -> K,
    ) -> Option<IVec3> {
        self.pending
            .keys()
            .copied()
            .min_by_key(|coord| key(*coord))
    }

    pub(crate) fn insert(&mut self, coord: IVec3, revision: u64, task: Task<T>) -> bool {
        if self.pending.contains_key(&coord) {
            return false;
        }

        self.pending.insert(
            coord,
            PendingChunkTask {
                revision: revision.into(),
                task,
            },
        );
        true
    }

    pub(crate) fn cancel_farthest_where(
        &mut self,
        center: IVec3,
        mut predicate: impl FnMut(IVec3) -> bool,
    ) -> Option<IVec3> {
        let coord = self
            .pending
            .keys()
            .copied()
            .filter(|coord| predicate(*coord))
            .max_by_key(|coord| {
                let delta = *coord - center;
                (delta.length_squared(), coord.x, coord.y, coord.z)
            })?;
        self.pending.remove(&coord);
        Some(coord)
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<T>> {
        let ready = self.pending.iter_mut().find_map(|(coord, pending)| {
            check_ready(&mut pending.task)
                .map(|output| (*coord, pending.revision, output))
        })?;
        let (coord, revision, output) = ready;
        self.pending.remove(&coord);

        Some(CompletedChunkTask {
            coord,
            revision,
            output,
        })
    }

    pub(crate) fn poll_ready_by_key<K: Ord>(
        &mut self,
        mut key: impl FnMut(IVec3) -> K,
    ) -> Option<CompletedChunkTask<T>> {
        let mut coords = self.pending.keys().copied().collect::<Vec<_>>();
        coords.sort_unstable_by_key(|coord| key(*coord));

        for coord in coords {
            let pending = self
                .pending
                .get_mut(&coord)
                .expect("selected chunk task must remain pending");
            let Some(output) = check_ready(&mut pending.task) else {
                continue;
            };
            let revision = pending.revision;
            self.pending.remove(&coord);

            return Some(CompletedChunkTask {
                coord,
                revision,
                output,
            });
        }

        None
    }
}
