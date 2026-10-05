use bevy::{platform::collections::HashSet, prelude::*};

use crate::voxel::{coordinates::ChunkCoord, deduplicated_queue::DeduplicatedQueue};

const MAX_RETIRED_SCAN_STEPS_PER_POLL: usize = 64;
const MAX_RETIRED_RESULTS_BEFORE_YIELD: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RetiredScanKey {
    center: IVec2,
    radius_squared: i64,
}

#[derive(Default)]
struct RetiredScanState {
    key: Option<RetiredScanKey>,
    queue_revision: u64,
    remaining: usize,
}

#[derive(Default)]
struct RetiredChunkQueue {
    queue: DeduplicatedQueue<ChunkCoord>,
    scan: RetiredScanState,
    returned_since_yield: usize,
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
        center: IVec2,
        radius_squared: i64,
        desired: &HashSet<IVec3>,
        retained: &HashSet<IVec3>,
    ) -> Option<IVec3> {
        if self.returned_since_yield >= MAX_RETIRED_RESULTS_BEFORE_YIELD {
            self.returned_since_yield = 0;
            return None;
        }

        let scan_key = RetiredScanKey {
            center,
            radius_squared,
        };
        let queue_revision = self.queue.revision();
        if self.scan.key != Some(scan_key) || self.scan.queue_revision != queue_revision {
            self.scan.key = Some(scan_key);
            self.scan.queue_revision = queue_revision;
            self.scan.remaining = self.queue.len();
        }

        let scan_steps = self.scan.remaining.min(MAX_RETIRED_SCAN_STEPS_PER_POLL);
        for _ in 0..scan_steps {
            let Some(coord) = self.queue.pop() else {
                self.scan.remaining = 0;
                self.scan.queue_revision = self.queue.revision();
                self.returned_since_yield = 0;
                return None;
            };
            self.scan.remaining -= 1;

            let world_coord = coord.as_ivec3();
            if desired.contains(&world_coord) || retained.contains(&world_coord) {
                self.scan.queue_revision = self.queue.revision();
                continue;
            }

            let delta_x = i64::from(world_coord.x) - i64::from(center.x);
            let delta_z = i64::from(world_coord.z) - i64::from(center.y);
            let outside_radius = delta_x * delta_x + delta_z * delta_z > radius_squared;
            if outside_radius {
                self.scan.queue_revision = self.queue.revision();
                self.returned_since_yield += 1;
                return Some(world_coord);
            }

            let enqueued = self.queue.enqueue(coord);
            debug_assert!(
                enqueued,
                "scanned retired chunk must re-enter the queue once"
            );
            self.scan.queue_revision = self.queue.revision();
        }

        self.returned_since_yield = 0;
        None
    }
}

/// Owns logical chunk residency independently from generation, presentation,
/// and Bevy entity lifetime.
#[derive(Default)]
pub(super) struct ChunkResidencyState {
    pub(super) desired: HashSet<IVec3>,
    pub(super) retained: HashSet<IVec3>,
    retired: RetiredChunkQueue,
}

impl ChunkResidencyState {
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
        let desired = &self.desired;
        let retained = &self.retained;
        self.retired
            .pop_outside_horizontal_radius(center, radius_squared, desired, retained)
    }
}

impl super::ChunkStreamingState {
    pub(in crate::world) fn diagnostic_retired_count(&self) -> usize {
        self.residency.retired_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_scan_progress_is_bounded_per_poll() {
        let mut queue = RetiredChunkQueue::default();
        let desired = HashSet::default();
        let retained = HashSet::default();
        let center = IVec2::ZERO;
        let radius_squared = 400;

        for x in 0..MAX_RETIRED_SCAN_STEPS_PER_POLL {
            queue.enqueue(IVec3::new(x as i32, 0, 0));
        }
        let far = IVec3::new(100, 0, 0);
        queue.enqueue(far);

        assert_eq!(
            queue.pop_outside_horizontal_radius(center, radius_squared, &desired, &retained),
            None,
        );
        assert_eq!(
            queue.pop_outside_horizontal_radius(center, radius_squared, &desired, &retained),
            Some(far),
        );
    }

    #[test]
    fn retired_results_force_a_yield_after_bounded_progress() {
        let mut queue = RetiredChunkQueue::default();
        let desired = HashSet::default();
        let retained = HashSet::default();
        let total = MAX_RETIRED_RESULTS_BEFORE_YIELD + 1;

        for x in 0..total {
            queue.enqueue(IVec3::new(100 + x as i32, 0, 0));
        }

        for _ in 0..MAX_RETIRED_RESULTS_BEFORE_YIELD {
            assert!(
                queue
                    .pop_outside_horizontal_radius(IVec2::ZERO, 0, &desired, &retained)
                    .is_some()
            );
        }
        assert_eq!(
            queue.pop_outside_horizontal_radius(IVec2::ZERO, 0, &desired, &retained),
            None,
        );
        assert!(
            queue
                .pop_outside_horizontal_radius(IVec2::ZERO, 0, &desired, &retained)
                .is_some()
        );
    }

    #[test]
    fn stale_retired_entries_are_discarded_when_selected_again() {
        let stale = IVec3::new(100, 0, 0);
        let eligible = IVec3::new(101, 0, 0);
        let mut queue = RetiredChunkQueue::default();
        queue.enqueue(stale);
        queue.enqueue(eligible);
        let mut desired = HashSet::default();
        desired.insert(stale);
        let retained = HashSet::default();

        assert_eq!(
            queue.pop_outside_horizontal_radius(IVec2::ZERO, 0, &desired, &retained),
            Some(eligible),
        );
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn retired_enqueue_restarts_an_exhausted_scan() {
        let near = IVec3::X;
        let far = IVec3::new(100, 0, 0);
        let mut queue = RetiredChunkQueue::default();
        queue.enqueue(near);
        let desired = HashSet::default();
        let retained = HashSet::default();

        assert_eq!(
            queue.pop_outside_horizontal_radius(IVec2::ZERO, 4, &desired, &retained),
            None,
        );
        queue.enqueue(far);
        assert_eq!(
            queue.pop_outside_horizontal_radius(IVec2::ZERO, 4, &desired, &retained),
            Some(far),
        );
    }
}
