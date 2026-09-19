use std::collections::HashMap;

use bevy::{
    prelude::IVec3,
    tasks::{Task, futures::check_ready},
};

pub(crate) struct CompletedChunkTask<T> {
    pub(crate) coord: IVec3,
    pub(crate) revision: u64,
    pub(crate) output: T,
}

struct PendingChunkTask<T> {
    revision: u64,
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

    pub(crate) fn any_coord(&self, mut predicate: impl FnMut(IVec3) -> bool) -> bool {
        self.pending.keys().copied().any(&mut predicate)
    }

    pub(crate) fn insert(&mut self, coord: IVec3, revision: u64, task: Task<T>) -> bool {
        if self.pending.contains_key(&coord) {
            return false;
        }

        self.pending
            .insert(coord, PendingChunkTask { revision, task });
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
}
