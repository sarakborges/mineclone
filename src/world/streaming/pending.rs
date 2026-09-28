use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bevy::prelude::IVec3;

use crate::voxel::deduplicated_queue::DeduplicatedQueue;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CriticalPendingScanKey {
    queue_revision: u64,
    center: IVec3,
}

#[derive(Default)]
struct PendingPriorityCache {
    queue_revision: u64,
    selection_revision: u64,
    pending: VecDeque<IVec3>,
}

/// Owns chunks waiting to enter generation work. Queue membership and the
/// caches derived from that membership stay together so the streaming
/// orchestrator does not need to coordinate revision invalidation itself.
#[derive(Default)]
pub(super) struct PendingChunkQueue {
    queue: DeduplicatedQueue<IVec3>,
    critical_scan_miss: Option<CriticalPendingScanKey>,
    priority_cache: PendingPriorityCache,
}

impl PendingChunkQueue {
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

    pub(super) fn clear(&mut self) {
        self.queue.clear();
    }

    pub(super) fn reserve(&mut self, additional: usize) {
        self.queue.reserve(additional);
    }

    pub(super) fn values(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.queue.values()
    }

    pub(super) fn values_in_order(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.queue.values_in_order()
    }

    pub(super) fn has_critical(
        &mut self,
        center: IVec3,
        mut is_critical: impl FnMut(IVec3) -> bool,
    ) -> bool {
        let scan_key = CriticalPendingScanKey {
            queue_revision: self.queue.revision(),
            center,
        };
        if self.critical_scan_miss == Some(scan_key) {
            return false;
        }

        let found = self.queue.values().any(&mut is_critical);
        self.critical_scan_miss = if found { None } else { Some(scan_key) };
        found
    }

    pub(super) fn pop_by_priority<K: Ord>(
        &mut self,
        selection_revision: u64,
        mut priority: impl FnMut(IVec3) -> K,
    ) -> (Option<IVec3>, Option<(Duration, usize)>) {
        let queue_revision = self.queue.revision();
        let mut scan = None;
        if self.priority_cache.queue_revision != queue_revision
            || self.priority_cache.selection_revision != selection_revision
        {
            let queue_len = self.queue.len();
            let started = Instant::now();
            let mut ordered = self.queue.values().collect::<Vec<_>>();
            ordered.sort_unstable_by_key(|coord| priority(*coord));
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
            debug_assert!(removed, "pending priority cache must reference an active chunk");
            self.priority_cache.queue_revision = self.queue.revision();
            return (Some(coord), scan);
        }

        (None, scan)
    }
}
