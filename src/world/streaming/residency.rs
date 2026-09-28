use bevy::{platform::collections::HashSet, prelude::*};

use crate::voxel::deduplicated_queue::DeduplicatedQueue;

#[derive(Default)]
pub(super) struct RetiredChunkQueue {
    queue: DeduplicatedQueue<IVec3>,
}

impl RetiredChunkQueue {
    pub(super) fn enqueue(&mut self, coord: IVec3) {
        self.queue.enqueue(coord);
    }

    pub(super) fn revision(&self) -> u64 {
        self.queue.revision()
    }

    pub(super) fn pop_where(&mut self, predicate: impl FnMut(IVec3) -> bool) -> Option<IVec3> {
        self.queue.pop_where(predicate)
    }
}

/// Owns logical chunk residency independently from generation, presentation,
/// and Bevy entity lifetime. Selection and retirement are world-runtime facts:
/// a chunk may remain resident without being generated, meshed, or visible.
#[derive(Default)]
pub(super) struct ChunkResidencyState {
    pub(super) desired: HashSet<IVec3>,
    pub(super) retained: HashSet<IVec3>,
    pub(super) retired: RetiredChunkQueue,
    revision: u64,
}

impl ChunkResidencyState {
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
    }

    pub(super) fn mark_rebuilt(&mut self) {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("chunk residency selection revision exhausted");
    }
}
