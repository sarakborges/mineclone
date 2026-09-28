use bevy::{platform::collections::HashSet, prelude::*};

/// Owns the logical chunk residency selection independently from the streaming
/// scheduler. A chunk may be logically resident without being generated,
/// meshed, visible, or backed by a Bevy entity.
#[derive(Default)]
pub(super) struct ChunkResidencyState {
    pub(super) desired: HashSet<IVec3>,
    pub(super) retained: HashSet<IVec3>,
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
