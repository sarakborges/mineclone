use bevy::{platform::collections::HashSet, prelude::*};

use crate::voxel::{
    coordinates::ChunkCoord,
    deduplicated_queue::DeduplicatedQueue,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ResidencySelectionRevision(u64);

impl ResidencySelectionRevision {
    fn next(self) -> Self {
        Self(
            self.0
                .checked_add(1)
                .expect("chunk residency selection revision exhausted"),
        )
    }

    pub(super) fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub(super) fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RetiredScanKey {
    queue_revision: u64,
    selection_revision: ResidencySelectionRevision,
    center: IVec2,
    radius_squared: i64,
}

#[derive(Default)]
struct RetiredChunkQueue {
    queue: DeduplicatedQueue<ChunkCoord>,
    scan_miss: Option<RetiredScanKey>,
}

impl RetiredChunkQueue {
    fn enqueue(&mut self, coord: IVec3) {
        self.queue.enqueue(ChunkCoord::from_ivec3(coord));
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn pop_outside_horizontal_radius(
        &mut self,
        selection_revision: ResidencySelectionRevision,
        center: IVec2,
        radius_squared: i64,
        desired: &HashSet<IVec3>,
        retained: &HashSet<IVec3>,
    ) -> Option<IVec3> {
        let scan_key = RetiredScanKey {
            queue_revision: self.queue.revision(),
            selection_revision,
            center,
            radius_squared,
        };
        if self.scan_miss == Some(scan_key) {
            return None;
        }

        let coord = self.queue.pop_where(|coord| {
            let coord = coord.as_ivec3();
            if desired.contains(&coord) || retained.contains(&coord) {
                return false;
            }

            let delta_x = i64::from(coord.x) - i64::from(center.x);
            let delta_z = i64::from(coord.z) - i64::from(center.y);
            delta_x * delta_x + delta_z * delta_z > radius_squared
        });
        self.scan_miss = if coord.is_some() {
            None
        } else {
            Some(scan_key)
        };
        coord.map(ChunkCoord::as_ivec3)
    }
}

/// Owns logical chunk residency independently from generation, presentation,
/// and Bevy entity lifetime. Selection and retirement are world-runtime facts:
/// a chunk may remain resident without being generated, meshed, or visible.
#[derive(Default)]
pub(super) struct ChunkResidencyState {
    pub(super) desired: HashSet<IVec3>,
    pub(super) retained: HashSet<IVec3>,
    retired: RetiredChunkQueue,
    revision: ResidencySelectionRevision,
}

impl ChunkResidencyState {
    pub(super) fn revision(&self) -> u64 {
        self.revision.raw()
    }

    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
    }

    pub(super) fn enqueue_retired(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.retired.enqueue(coord);
        }
    }

    pub(super) fn retired_len(&self) -> usize {
        self.retired.len()
    }

    pub(super) fn pop_retired_outside_horizontal_radius(
        &mut self,
        center: IVec3,
        horizontal_radius: i32,
    ) -> Option<IVec3> {
        let center = center.xz();
        let radius = i64::from(horizontal_radius.max(0));
        let radius_squared = radius * radius;
        let selection_revision = self.revision;
        let desired = &self.desired;
        let retained = &self.retained;
        self.retired.pop_outside_horizontal_radius(
            selection_revision,
            center,
            radius_squared,
            desired,
            retained,
        )
    }

    pub(super) fn mark_rebuilt(&mut self) {
        self.revision = self.revision.next();
    }
}

impl super::ChunkStreamingState {
    pub(super) fn diagnostic_retired_count(&self) -> usize {
        self.residency.retired_len()
    }
}
