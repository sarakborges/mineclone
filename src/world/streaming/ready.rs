use std::time::{Duration, Instant};

use bevy::prelude::{IVec2, IVec3};

use crate::voxel::{
    coordinates::ChunkCoord,
    deduplicated_queue::DeduplicatedQueue,
};

use super::residency::ResidencySelectionRevision;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReadyScanKey {
    queue_revision: u64,
    selection_revision: ResidencySelectionRevision,
    center: IVec2,
    radius_squared: i64,
}

/// Owns chunks waiting for initial presentation. The negative scan cache is
/// tied to the queue revision and residency-selection revision, so stale misses
/// cannot survive either queue mutation or a new residency selection.
#[derive(Default)]
pub(super) struct ReadyChunkQueue {
    queue: DeduplicatedQueue<ChunkCoord>,
    scan_miss: Option<ReadyScanKey>,
}

impl ReadyChunkQueue {
    pub(super) fn len(&self) -> usize {
        self.queue.len()
    }

    pub(super) fn contains(&self, coord: IVec3) -> bool {
        self.queue.contains(ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn enqueue(&mut self, coord: IVec3) {
        self.queue.enqueue(ChunkCoord::from_ivec3(coord));
    }

    pub(super) fn enqueue_front(&mut self, coord: IVec3) {
        self.queue.enqueue_front(ChunkCoord::from_ivec3(coord));
    }

    pub(super) fn remove(&mut self, coord: IVec3) -> bool {
        self.queue.remove(ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn retain(&mut self, mut predicate: impl FnMut(IVec3) -> bool) -> usize {
        let removed = self
            .queue
            .values()
            .filter(|coord| !predicate(coord.as_ivec3()))
            .collect::<Vec<_>>();
        let removed_count = removed.len();
        for coord in removed {
            let did_remove = self.queue.remove(coord);
            debug_assert!(did_remove, "ready retention selected an active queue entry");
        }
        removed_count
    }

    pub(super) fn values(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.queue.values().map(ChunkCoord::as_ivec3)
    }

    pub(super) fn pop_min_where_by_key<K: Ord>(
        &mut self,
        selection_revision: u64,
        center: IVec2,
        radius_squared: i64,
        mut predicate: impl FnMut(IVec3) -> bool,
        mut key: impl FnMut(IVec3) -> K,
    ) -> (Option<IVec3>, Option<(Duration, usize)>) {
        let selection_revision = ResidencySelectionRevision::from_raw(selection_revision);
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
        let selected = self.queue.pop_min_where_by_key(
            |coord| predicate(coord.as_ivec3()),
            |coord| key(coord.as_ivec3()),
        );
        let scan = Some((started.elapsed(), queue_len));
        self.scan_miss = if selected.is_some() {
            None
        } else {
            Some(scan_key)
        };
        (selected.map(ChunkCoord::as_ivec3), scan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_removes_only_stale_ready_chunks() {
        let stale = IVec3::new(-4, 0, 0);
        let desired = IVec3::new(2, 0, 0);
        let retained = IVec3::new(5, 0, 0);
        let mut queue = ReadyChunkQueue::default();
        queue.enqueue(stale);
        queue.enqueue(desired);
        queue.enqueue(retained);

        assert_eq!(
            queue.retain(|coord| coord == desired || coord == retained),
            1
        );
        assert!(!queue.contains(stale));
        assert!(queue.contains(desired));
        assert!(queue.contains(retained));
        assert_eq!(queue.len(), 2);
    }
}
