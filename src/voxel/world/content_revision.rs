use bevy::{platform::collections::HashMap, prelude::IVec3};

use crate::voxel::revision::ChunkContentRevision;

#[derive(Clone, Default)]
pub(super) struct ContentRevisionState {
    by_chunk: HashMap<IVec3, ChunkContentRevision>,
    next: ChunkContentRevision,
}

impl ContentRevisionState {
    pub(super) fn revision(&self, coord: IVec3) -> Option<ChunkContentRevision> {
        self.by_chunk.get(&coord).copied()
    }

    pub(super) fn mark_changed(&mut self, coord: IVec3) {
        self.next = self
            .next
            .checked_next()
            .expect("chunk content revision counter exhausted");
        self.by_chunk.insert(coord, self.next);
    }

    pub(super) fn remove_chunk(&mut self, coord: IVec3) -> Option<ChunkContentRevision> {
        self.by_chunk.remove(&coord)
    }
}
