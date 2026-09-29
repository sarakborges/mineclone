use bevy::{platform::collections::HashSet, prelude::*};

use crate::voxel::{
    coordinates::ChunkCoord,
    deduplicated_queue::DeduplicatedQueue,
};

const MAX_RETIRED_SCAN_STEPS_PER_POLL: usize = 16;

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
    selection_revision: ResidencySelectionRevision,
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
            selection_revision,
            center,
            radius_squared,
        };
        let queue_revision = self.queue.revision();
        if self.scan.key != Some(scan_key) || self.scan.queue_revision != queue_revision {
            self.scan.key = Some(scan_key);
            self.scan.queue_revision = queue_revision;
            self.scan.remaining = self.queue.len();
        }

        let scan_steps = self
            .scan
            .remaining
            .min(MAX_RETIRED_SCAN_STEPS_PER_POLL);
        for _ in 0..scan_steps {
            let Some(coord) = self.queue.pop() else {
                self.scan.remaining = 0;
                self.scan.queue_revision = self.queue.revision();
                return None;
            };
            self.scan.remaining -= 1;

            let world_coord = coord.as_ivec3();
            let delta_x = i64::from(world_coord.x) - i64::from(center.x);
            let delta_z = i64::from(world_coord.z) - i64::from(center.y);
            let outside_radius = delta_x * delta_x + delta_z * delta_z > radius_squared;
            if outside_radius
                && !desired.contains(&world_coord)
                && !retained.contains(&world_coord)
            {
                self.scan.queue_revision = self.queue.revision();
                return Some(world_coord);
            }

            let enqueued = self.queue.enqueue(coord);
            debug_assert!(enqueued, "scanned retired chunk must re-enter the queue once");
            self.scan.queue_revision = self.queue.revision();
        }

        None
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
            queue.pop_outside_horizontal_radius(
                ResidencySelectionRevision::default(),
                center,
                radius_squared,
                &desired,
                &retained,
            ),
            None,
        );
        assert_eq!(
            queue.pop_outside_horizontal_radius(
                ResidencySelectionRevision::default(),
                center,
                radius_squared,
                &desired,
                &retained,
            ),
            Some(far),
        );
    }

    #[test]
    fn exhausted_retired_scan_restarts_after_selection_revision_change() {
        let coord = IVec3::new(5, 0, 0);
        let mut queue = RetiredChunkQueue::default();
        queue.enqueue(coord);
        let desired = HashSet::default();
        let mut retained = HashSet::default();
        retained.insert(coord);
        let revision = ResidencySelectionRevision::default();

        assert_eq!(
            queue.pop_outside_horizontal_radius(
                revision,
                IVec2::ZERO,
                0,
                &desired,
                &retained,
            ),
            None,
        );
        retained.remove(&coord);
        assert_eq!(
            queue.pop_outside_horizontal_radius(
                revision,
                IVec2::ZERO,
                0,
                &desired,
                &retained,
            ),
            None,
        );
        assert_eq!(
            queue.pop_outside_horizontal_radius(
                revision.next(),
                IVec2::ZERO,
                0,
                &desired,
                &retained,
            ),
            Some(coord),
        );
    }

    #[test]
    fn retired_enqueue_restarts_an_exhausted_scan() {
        let near = IVec3::X;
        let far = IVec3::new(100, 0, 0);
        let mut queue = RetiredChunkQueue::default();
        queue.enqueue(near);
        let desired = HashSet::default();
        let retained = HashSet::default();
        let revision = ResidencySelectionRevision::default();

        assert_eq!(
            queue.pop_outside_horizontal_radius(
                revision,
                IVec2::ZERO,
                4,
                &desired,
                &retained,
            ),
            None,
        );
        queue.enqueue(far);
        assert_eq!(
            queue.pop_outside_horizontal_radius(
                revision,
                IVec2::ZERO,
                4,
                &desired,
                &retained,
            ),
            Some(far),
        );
    }
}
