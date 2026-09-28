use bevy::{platform::collections::HashSet, prelude::*};

use crate::voxel::deduplicated_queue::DeduplicatedQueue;

/// Owns logical chunk residency independently from generation, presentation,
/// and Bevy entity lifetime. Selection and retirement are world-runtime facts:
/// a chunk may remain resident without being generated, meshed, or visible.
#[derive(Default)]
pub(super) struct ChunkResidencyState {
    pub(super) desired: HashSet<IVec3>,
    pub(super) retained: HashSet<IVec3>,
    pub(super) retired: DeduplicatedQueue<IVec3>,
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
