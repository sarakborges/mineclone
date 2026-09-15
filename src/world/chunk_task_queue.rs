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

    pub(crate) fn insert(&mut self, coord: IVec3, revision: u64, task: Task<T>) -> bool {
        if self.pending.contains_key(&coord) {
            return false;
        }

        self.pending
            .insert(coord, PendingChunkTask { revision, task });
        true
    }

    pub(crate) fn collect_ready(&mut self, maximum: usize) -> Vec<CompletedChunkTask<T>> {
        if maximum == 0 || self.pending.is_empty() {
            return Vec::new();
        }

        let coords = self.pending.keys().copied().collect::<Vec<_>>();
        let mut completed = Vec::new();

        for coord in coords {
            if completed.len() >= maximum {
                break;
            }

            let ready = {
                let pending = self
                    .pending
                    .get_mut(&coord)
                    .unwrap_or_else(|| panic!("pending chunk task disappeared for {coord:?}"));
                check_ready(&mut pending.task).map(|output| (pending.revision, output))
            };

            let Some((revision, output)) = ready else {
                continue;
            };
            self.pending.remove(&coord);
            completed.push(CompletedChunkTask {
                coord,
                revision,
                output,
            });
        }

        completed
    }
}
