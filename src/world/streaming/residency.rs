use std::ops::{Deref, DerefMut};

use bevy::{platform::collections::HashSet, prelude::*};

use super::ChunkStreamingState;

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

// Migration bridge: selection code still reads `streaming.desired` and
// `streaming.retained` while residency ownership moves out of the scheduler.
// Remove these impls once the remaining queue/selection call sites consume the
// residency owner explicitly.
impl Deref for ChunkStreamingState {
    type Target = ChunkResidencyState;

    fn deref(&self) -> &Self::Target {
        &self.residency
    }
}

impl DerefMut for ChunkStreamingState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.residency
    }
}
