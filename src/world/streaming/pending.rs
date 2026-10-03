use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bevy::prelude::IVec3;

use crate::voxel::{coordinates::ChunkCoord, deduplicated_queue::DeduplicatedQueue};

use super::residency::ResidencySelectionRevision;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CriticalPendingScanKey {
    queue_revision: u64,
    center: ChunkCoord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CriticalPendingScanResult {
    key: CriticalPendingScanKey,
    found: bool,
}

#[derive(Default)]
struct PendingPriorityCache {
    queue_revision: u64,
    selection_revision: ResidencySelectionRevision,
    pending: VecDeque<ChunkCoord>,
}

/// Owns chunks waiting to enter generation work. Queue membership and the
/// caches derived from that membership stay together so the streaming
/// orchestrator does not need to coordinate revision invalidation itself.
#[derive(Default)]
pub(super) struct PendingChunkQueue {
    queue: DeduplicatedQueue<ChunkCoord>,
    critical_scan: Option<CriticalPendingScanResult>,
    priority_cache: PendingPriorityCache,
}

impl PendingChunkQueue {
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

    pub(super) fn clear(&mut self) {
        self.queue.clear();
    }

    pub(super) fn reserve(&mut self, additional: usize) {
        self.queue.reserve(additional);
    }

    pub(super) fn values(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.queue.values().map(ChunkCoord::as_ivec3)
    }

    #[cfg(test)]
    pub(super) fn values_in_order(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.queue.values_in_order().map(ChunkCoord::as_ivec3)
    }

    pub(super) fn has_critical(
        &mut self,
        center: IVec3,
        mut is_critical: impl FnMut(IVec3) -> bool,
    ) -> bool {
        let scan_key = CriticalPendingScanKey {
            queue_revision: self.queue.revision(),
            center: ChunkCoord::from_ivec3(center),
        };
        if let Some(cached) = self.critical_scan
            && cached.key == scan_key
        {
            return cached.found;
        }

        let found = self
            .queue
            .values()
            .any(|coord| is_critical(coord.as_ivec3()));
        self.critical_scan = Some(CriticalPendingScanResult {
            key: scan_key,
            found,
        });
        found
    }

    pub(super) fn pop_by_priority<K: Ord>(
        &mut self,
        selection_revision: u64,
        mut priority: impl FnMut(IVec3) -> K,
    ) -> (Option<IVec3>, Option<(Duration, usize)>) {
        let selection_revision = ResidencySelectionRevision::from_raw(selection_revision);
        let queue_revision = self.queue.revision();
        let mut scan = None;
        if self.priority_cache.queue_revision != queue_revision
            || self.priority_cache.selection_revision != selection_revision
        {
            let queue_len = self.queue.len();
            let started = Instant::now();
            let mut ordered = self.queue.values().collect::<Vec<_>>();
            ordered.sort_unstable_by_key(|coord| priority(coord.as_ivec3()));
            scan = Some((started.elapsed(), queue_len));
            self.priority_cache.pending = ordered.into();
            self.priority_cache.queue_revision = queue_revision;
            self.priority_cache.selection_revision = selection_revision;
        }

        while let Some(coord) = self.priority_cache.pending.pop_front() {
            if !self.queue.contains(coord) {
                continue;
            }

            let removed = self.queue.remove(coord);
            debug_assert!(
                removed,
                "pending priority cache must reference an active chunk"
            );
            self.priority_cache.queue_revision = self.queue.revision();
            return (Some(coord.as_ivec3()), scan);
        }

        (None, scan)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn critical_scan_caches_positive_and_negative_results_until_membership_changes() {
        let critical = IVec3::X;
        let noncritical = IVec3::new(4, 0, 0);
        let mut queue = PendingChunkQueue::default();
        queue.enqueue(critical);
        queue.enqueue(noncritical);

        let predicate_calls = Cell::new(0_usize);
        let is_critical = |coord: IVec3| {
            predicate_calls.set(predicate_calls.get() + 1);
            coord == critical
        };

        assert!(queue.has_critical(IVec3::ZERO, is_critical));
        let calls_after_first_scan = predicate_calls.get();
        assert!(calls_after_first_scan > 0);

        assert!(queue.has_critical(IVec3::ZERO, is_critical));
        assert_eq!(predicate_calls.get(), calls_after_first_scan);

        assert!(queue.remove(critical));
        assert!(!queue.has_critical(IVec3::ZERO, is_critical));
        let calls_after_membership_change = predicate_calls.get();
        assert!(calls_after_membership_change > calls_after_first_scan);

        assert!(!queue.has_critical(IVec3::ZERO, is_critical));
        assert_eq!(predicate_calls.get(), calls_after_membership_change);
    }
}
