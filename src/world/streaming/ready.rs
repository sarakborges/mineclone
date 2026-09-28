use std::time::{Duration, Instant};

use bevy::prelude::{IVec2, IVec3};

use crate::voxel::deduplicated_queue::DeduplicatedQueue;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReadyScanKey {
    queue_revision: u64,
    selection_revision: u64,
    center: IVec2,
    radius_squared: i64,
}

/// Owns chunks waiting for initial presentation. The negative scan cache is
/// tied to the queue revision and residency-selection revision, so stale misses
/// cannot survive either queue mutation or a new residency selection.
#[derive(Default)]
pub(super) struct ReadyChunkQueue {
    queue: DeduplicatedQueue<IVec3>,
    scan_miss: Option<ReadyScanKey>,
}

impl ReadyChunkQueue {
    pub(super) fn len(&self) -> usize {
        self.queue.len()
    }

    pub(super) fn contains(&self, coord: IVec3) -> bool {
        self.queue.contains(coord)
    }

    pub(super) fn enqueue(&mut self, coord: IVec3) {
        self.queue.enqueue(coord);
    }

    pub(super) fn enqueue_front(&mut self, coord: IVec3) {
        self.queue.enqueue_front(coord);
    }

    pub(super) fn remove(&mut self, coord: IVec3) -> bool {
        self.queue.remove(coord)
    }

    pub(super) fn values(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.queue.values()
    }

    pub(super) fn pop_min_where_by_key<K: Ord>(
        &mut self,
        selection_revision: u64,
        center: IVec2,
        radius_squared: i64,
        predicate: impl FnMut(IVec3) -> bool,
        key: impl FnMut(IVec3) -> K,
    ) -> (Option<IVec3>, Option<(Duration, usize)>) {
        let scan_key = ReadyScanKey {
            queue_revision: self.queue.revision(),
            selection_revision,
            center,
            radius_squared,
        };
        if self.scan_miss == Some(scan_key) {
            return (None, None);
        }

        let queue_len = self.queue.len();
        let started = Instant::now();
        let selected = self.queue.pop_min_where_by_key(predicate, key);
        let scan = Some((started.elapsed(), queue_len));
        self.scan_miss = if selected.is_some() {
            None
        } else {
            Some(scan_key)
        };
        (selected, scan)
    }

    #[cfg(test)]
    pub(super) fn scan_is_cached(&self) -> bool {
        self.scan_miss.is_some()
    }
}
