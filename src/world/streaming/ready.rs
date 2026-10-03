use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{IVec2, IVec3},
};

use crate::voxel::{coordinates::ChunkCoord, deduplicated_queue::DeduplicatedQueue};

use super::residency::ResidencySelectionRevision;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReadyScanKey {
    queue_revision: u64,
    selection_revision: ResidencySelectionRevision,
    center: IVec2,
    radius_squared: i64,
}

#[derive(Default)]
struct ReadyPriorityCache {
    key: Option<ReadyScanKey>,
    pending: VecDeque<ChunkCoord>,
}

/// Owns chunks waiting for initial presentation. Priority ordering is cached
/// against both queue membership and residency-selection state so repeated
/// publication pops do not rescan the whole ready queue.
#[derive(Default)]
pub(super) struct ReadyChunkQueue {
    queue: DeduplicatedQueue<ChunkCoord>,
    columns: HashMap<IVec2, HashSet<ChunkCoord>>,
    priority_cache: ReadyPriorityCache,
}

impl ReadyChunkQueue {
    pub(super) fn len(&self) -> usize {
        self.queue.len()
    }

    pub(super) fn contains(&self, coord: IVec3) -> bool {
        self.queue.contains(ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn enqueue(&mut self, coord: IVec3) {
        let coord = ChunkCoord::from_ivec3(coord);
        if self.queue.enqueue(coord) {
            self.index_insert(coord);
        }
    }

    pub(super) fn enqueue_front(&mut self, coord: IVec3) {
        let coord = ChunkCoord::from_ivec3(coord);
        let was_present = self.queue.contains(coord);
        self.queue.enqueue_front(coord);
        if !was_present {
            self.index_insert(coord);
        }
    }

    pub(super) fn remove(&mut self, coord: IVec3) -> bool {
        self.remove_chunk_coord(ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn retain(&mut self, mut predicate: impl FnMut(IVec3) -> bool) -> usize {
        let removed = self
            .queue
            .values()
            .filter(|coord| !predicate(coord.as_ivec3()))
            .collect::<Vec<_>>();
        let removed_count = removed.len();
        for coord in removed {
            let did_remove = self.remove_chunk_coord(coord);
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
        let mut scan = None;
        if self.priority_cache.key != Some(scan_key) {
            let queue_len = self.queue.len();
            let started = Instant::now();
            let mut ordered = self.visible_candidates(center, radius_squared, &mut predicate);
            ordered.sort_unstable_by_key(|coord| key(coord.as_ivec3()));
            scan = Some((started.elapsed(), queue_len));
            self.priority_cache.pending = ordered.into();
            self.priority_cache.key = Some(scan_key);
        }

        while let Some(coord) = self.priority_cache.pending.pop_front() {
            if !self.queue.contains(coord) {
                continue;
            }

            let removed = self.remove_chunk_coord(coord);
            debug_assert!(
                removed,
                "ready priority cache must reference an active chunk"
            );
            if let Some(cache_key) = self.priority_cache.key.as_mut() {
                cache_key.queue_revision = self.queue.revision();
            }
            return (Some(coord.as_ivec3()), scan);
        }

        (None, scan)
    }

    fn visible_candidates(
        &self,
        center: IVec2,
        radius_squared: i64,
        predicate: &mut impl FnMut(IVec3) -> bool,
    ) -> Vec<ChunkCoord> {
        let radius_squared = radius_squared.max(0);
        let radius = (radius_squared as f64).sqrt().floor() as i32;
        let mut candidates = Vec::new();

        for z in center.y.saturating_sub(radius)..=center.y.saturating_add(radius) {
            let dz = i64::from(z) - i64::from(center.y);
            let remaining = radius_squared - dz * dz;
            if remaining < 0 {
                continue;
            }
            let x_span = (remaining as f64).sqrt().floor() as i32;
            for x in center.x.saturating_sub(x_span)..=center.x.saturating_add(x_span) {
                let horizontal = IVec2::new(x, z);
                let Some(column) = self.columns.get(&horizontal) else {
                    continue;
                };
                candidates.extend(
                    column
                        .iter()
                        .copied()
                        .filter(|coord| predicate(coord.as_ivec3())),
                );
            }
        }

        candidates
    }

    fn index_insert(&mut self, coord: ChunkCoord) {
        let position = coord.as_ivec3();
        let horizontal = IVec2::new(position.x, position.z);
        self.columns.entry(horizontal).or_default().insert(coord);
    }

    fn remove_chunk_coord(&mut self, coord: ChunkCoord) -> bool {
        if !self.queue.remove(coord) {
            return false;
        }

        let position = coord.as_ivec3();
        let horizontal = IVec2::new(position.x, position.z);
        let column = self
            .columns
            .get_mut(&horizontal)
            .expect("ready column index must contain queued column");
        let indexed = column.remove(&coord);
        debug_assert!(indexed, "ready column index must contain queued chunk");
        let remove_column = column.is_empty();
        if remove_column {
            self.columns.remove(&horizontal);
        }
        true
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

    #[test]
    fn priority_cache_survives_its_own_queue_pops() {
        let near = IVec3::X;
        let middle = IVec3::new(2, 0, 0);
        let far = IVec3::new(3, 0, 0);
        let mut queue = ReadyChunkQueue::default();
        queue.enqueue(far);
        queue.enqueue(near);
        queue.enqueue(middle);

        let (first, first_scan) = queue.pop_min_where_by_key(
            1,
            IVec2::ZERO,
            144,
            |_| true,
            |coord| coord.length_squared(),
        );
        let (second, second_scan) = queue.pop_min_where_by_key(
            1,
            IVec2::ZERO,
            144,
            |_| true,
            |coord| coord.length_squared(),
        );
        let (third, third_scan) = queue.pop_min_where_by_key(
            1,
            IVec2::ZERO,
            144,
            |_| true,
            |coord| coord.length_squared(),
        );

        assert_eq!(first, Some(near));
        assert!(first_scan.is_some());
        assert_eq!(second, Some(middle));
        assert!(second_scan.is_none());
        assert_eq!(third, Some(far));
        assert!(third_scan.is_none());
    }

    #[test]
    fn selection_change_rebuilds_cached_eligibility() {
        let old_visible = IVec3::X;
        let newly_visible = IVec3::new(10, 0, 0);
        let mut queue = ReadyChunkQueue::default();
        queue.enqueue(old_visible);
        queue.enqueue(newly_visible);

        let (selected, scan) = queue.pop_min_where_by_key(
            1,
            IVec2::ZERO,
            4,
            |coord| coord == old_visible,
            |coord| coord.length_squared(),
        );
        assert_eq!(selected, Some(old_visible));
        assert!(scan.is_some());

        let (selected, scan) = queue.pop_min_where_by_key(
            2,
            IVec2::new(10, 0),
            4,
            |coord| coord == newly_visible,
            |coord| (coord - newly_visible).length_squared(),
        );
        assert_eq!(selected, Some(newly_visible));
        assert!(scan.is_some());
    }

    #[test]
    fn visible_scan_ignores_ready_columns_outside_radius() {
        let visible = IVec3::new(2, 3, 1);
        let outside = IVec3::new(20, 0, 20);
        let mut queue = ReadyChunkQueue::default();
        queue.enqueue(outside);
        queue.enqueue(visible);

        let mut predicate_calls = 0;
        let (selected, scan) = queue.pop_min_where_by_key(
            1,
            IVec2::ZERO,
            9,
            |_| {
                predicate_calls += 1;
                true
            },
            |coord| coord.length_squared(),
        );

        assert_eq!(selected, Some(visible));
        assert!(scan.is_some());
        assert_eq!(predicate_calls, 1);
        assert!(queue.contains(outside));
    }

    #[test]
    fn column_index_stays_coherent_after_retain_and_front_requeue() {
        let retained = IVec3::new(1, 2, 1);
        let removed = IVec3::new(2, 2, 2);
        let mut queue = ReadyChunkQueue::default();
        queue.enqueue(retained);
        queue.enqueue(removed);
        queue.enqueue_front(retained);

        assert_eq!(queue.retain(|coord| coord == retained), 1);
        let (selected, _) =
            queue.pop_min_where_by_key(1, IVec2::ZERO, 9, |_| true, |coord| coord.length_squared());
        assert_eq!(selected, Some(retained));
        assert_eq!(queue.len(), 0);
        assert!(queue.columns.is_empty());
    }
}
