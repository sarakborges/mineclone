use std::collections::HashMap;

use bevy::tasks::{Task, futures::check_ready};

use crate::voxel::coordinates::ChunkCoord;

use super::revision::TaskInputRevision;

pub(crate) struct CompletedChunkTask<T, C = bevy::prelude::IVec3> {
    pub(crate) coord: C,
    pub(crate) revision: TaskInputRevision,
    pub(crate) output: T,
}

impl<T> CompletedChunkTask<T, ChunkCoord> {
    pub(crate) fn into_runtime(self) -> CompletedChunkTask<T> {
        CompletedChunkTask {
            coord: self.coord.as_ivec3(),
            revision: self.revision,
            output: self.output,
        }
    }
}

struct PendingChunkTask<T> {
    revision: TaskInputRevision,
    task: Task<T>,
}

pub(crate) struct ChunkTaskQueue<T> {
    pending: HashMap<ChunkCoord, PendingChunkTask<T>>,
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

    pub(crate) fn contains(&self, coord: ChunkCoord) -> bool {
        self.pending.contains_key(&coord)
    }

    pub(crate) fn cancel(&mut self, coord: ChunkCoord) -> bool {
        self.pending.remove(&coord).is_some()
    }

    pub(crate) fn cancel_where(
        &mut self,
        mut predicate: impl FnMut(ChunkCoord) -> bool,
    ) -> Vec<ChunkCoord> {
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
        mut key: impl FnMut(ChunkCoord) -> K,
    ) -> Option<ChunkCoord> {
        self.pending.keys().copied().min_by_key(|coord| key(*coord))
    }

    pub(crate) fn insert(
        &mut self,
        coord: ChunkCoord,
        revision: TaskInputRevision,
        task: Task<T>,
    ) -> bool {
        if self.pending.contains_key(&coord) {
            return false;
        }

        self.pending
            .insert(coord, PendingChunkTask { revision, task });
        true
    }

    pub(crate) fn cancel_farthest_where(
        &mut self,
        center: ChunkCoord,
        mut predicate: impl FnMut(ChunkCoord) -> bool,
    ) -> Option<ChunkCoord> {
        let center = center.as_ivec3();
        let coord = self
            .pending
            .keys()
            .copied()
            .filter(|coord| predicate(*coord))
            .max_by_key(|coord| {
                let coord = coord.as_ivec3();
                let delta = coord - center;
                (delta.length_squared(), coord.x, coord.y, coord.z)
            })?;
        self.pending.remove(&coord);
        Some(coord)
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<T, ChunkCoord>> {
        let ready = self.pending.iter_mut().find_map(|(coord, pending)| {
            check_ready(&mut pending.task).map(|output| (*coord, pending.revision, output))
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
        mut key: impl FnMut(ChunkCoord) -> K,
    ) -> Option<CompletedChunkTask<T, ChunkCoord>> {
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
