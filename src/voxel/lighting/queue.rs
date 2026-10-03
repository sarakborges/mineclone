use bevy::prelude::*;

use crate::voxel::{
    chunk::{CHUNK_SIZE, CHUNK_VOLUME},
    deduplicated_queue::DeduplicatedQueue,
    neighbors::CARDINAL_NEIGHBORS,
    update_queue::VoxelUpdateQueue,
};

const CHUNK_INTERIOR_VOLUME: usize = (CHUNK_SIZE - 2) * (CHUNK_SIZE - 2) * (CHUNK_SIZE - 2);
const CHUNK_BOUNDARY_VOXEL_COUNT: usize = CHUNK_VOLUME - CHUNK_INTERIOR_VOLUME;
const CHUNK_BOUNDARY_NEIGHBOR_COUNT: usize = 6 * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LightingLane {
    Interactive,
    Settling,
    Background,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum DeferredBackgroundScanRequest {
    ChunkVoxels(IVec3),
    ChunkBoundaryNeighbors(IVec3),
}

impl DeferredBackgroundScanRequest {
    fn len(self) -> usize {
        match self {
            Self::ChunkVoxels(_) => CHUNK_VOLUME,
            Self::ChunkBoundaryNeighbors(_) => CHUNK_BOUNDARY_NEIGHBOR_COUNT,
        }
    }

    fn position(self, index: usize) -> IVec3 {
        match self {
            Self::ChunkVoxels(origin) => chunk_voxel_position(origin, index),
            Self::ChunkBoundaryNeighbors(origin) => chunk_boundary_neighbor_position(origin, index),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DeferredBackgroundScan {
    request: DeferredBackgroundScanRequest,
    next_index: usize,
}

#[derive(Default)]
struct DeferredBackgroundScanQueue {
    pending: DeduplicatedQueue<DeferredBackgroundScanRequest>,
    active: Option<DeferredBackgroundScan>,
}

impl DeferredBackgroundScanQueue {
    fn enqueue(&mut self, request: DeferredBackgroundScanRequest) -> bool {
        self.pending.enqueue(request)
    }

    fn has_active(&self) -> bool {
        self.active.is_some()
    }

    fn has_work(&self) -> bool {
        self.active.is_some() || self.pending.len() > 0
    }

    fn pop(&mut self) -> Option<IVec3> {
        loop {
            if self.active.is_none() {
                let request = self.pending.pop()?;
                self.active = Some(DeferredBackgroundScan {
                    request,
                    next_index: 0,
                });
            }

            let scan = self
                .active
                .as_mut()
                .expect("deferred background scan must be active after dequeue");
            let position = scan.request.position(scan.next_index);
            scan.next_index += 1;
            if scan.next_index == scan.request.len() {
                self.active = None;
            }

            // VoxelUpdateQueue::enqueue rejects negative world Y. Deferred
            // scans preserve the same semantic without materializing those
            // rejected entries eagerly.
            if position.y >= 0 {
                return Some(position);
            }
        }
    }
}

fn chunk_voxel_position(origin: IVec3, index: usize) -> IVec3 {
    let plane = CHUNK_SIZE * CHUNK_SIZE;
    let y = index / plane;
    let remainder = index % plane;
    let z = remainder / CHUNK_SIZE;
    let x = remainder % CHUNK_SIZE;
    origin + IVec3::new(x as i32, y as i32, z as i32)
}

fn chunk_boundary_neighbor_position(origin: IVec3, index: usize) -> IVec3 {
    let size = CHUNK_SIZE as i32;
    let face_area = CHUNK_SIZE * CHUNK_SIZE;
    let face_pair_span = face_area * 2;

    if index < face_pair_span {
        let pair = index / 2;
        let y = pair / CHUNK_SIZE;
        let z = pair % CHUNK_SIZE;
        let x = if index.is_multiple_of(2) { -1 } else { size };
        return origin + IVec3::new(x, y as i32, z as i32);
    }

    if index < face_pair_span * 2 {
        let local = index - face_pair_span;
        let pair = local / 2;
        let y = pair / CHUNK_SIZE;
        let x = pair % CHUNK_SIZE;
        let z = if local.is_multiple_of(2) { -1 } else { size };
        return origin + IVec3::new(x as i32, y as i32, z);
    }

    let local = index - face_pair_span * 2;
    let pair = local / 2;
    let z = pair / CHUNK_SIZE;
    let x = pair % CHUNK_SIZE;
    let y = if local.is_multiple_of(2) { -1 } else { size };
    origin + IVec3::new(x as i32, y, z as i32)
}

#[derive(Default)]
pub(super) struct LightingQueue {
    interactive: VoxelUpdateQueue,
    settling: VoxelUpdateQueue,
    background: VoxelUpdateQueue,
    deferred_background_scans: DeferredBackgroundScanQueue,
    background_items_before_deferred: usize,
}

impl LightingQueue {
    pub fn enqueue(&mut self, position: IVec3) {
        if self.interactive.contains(position) || self.settling.contains(position) {
            return;
        }
        self.background.enqueue(position);
    }

    pub(super) fn enqueue_interactive(&mut self, position: IVec3) {
        self.background.remove(position);
        self.settling.remove(position);
        self.interactive.enqueue(position);
    }

    fn enqueue_interactive_priority(&mut self, position: IVec3) {
        self.background.remove(position);
        self.settling.remove(position);
        self.interactive.enqueue_priority(position);
    }

    pub(super) fn enqueue_settling(&mut self, position: IVec3) {
        if self.interactive.contains(position) {
            return;
        }
        self.background.remove(position);
        self.settling.enqueue(position);
    }

    pub fn enqueue_with_neighbors(&mut self, position: IVec3) {
        self.enqueue(position);
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue(position + offset);
        }
    }

    pub fn enqueue_with_neighbors_priority(&mut self, position: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_interactive_priority(position + offset);
        }
        self.enqueue_interactive_priority(position);
    }

    pub(super) fn enqueue_with_neighbors_in_lane(&mut self, position: IVec3, lane: LightingLane) {
        match lane {
            LightingLane::Interactive => {
                self.enqueue_interactive(position);
                for offset in CARDINAL_NEIGHBORS {
                    self.enqueue_interactive(position + offset);
                }
            }
            LightingLane::Settling => {
                self.enqueue_settling(position);
                for offset in CARDINAL_NEIGHBORS {
                    self.enqueue_settling(position + offset);
                }
            }
            LightingLane::Background => self.enqueue_with_neighbors(position),
        }
    }

    pub fn enqueue_chunk_voxels(&mut self, origin: IVec3) {
        if origin.y < 0 {
            return;
        }

        self.enqueue_deferred_background_scan(DeferredBackgroundScanRequest::ChunkVoxels(origin));
    }

    pub fn enqueue_chunk_boundary_voxels(&mut self, origin: IVec3) {
        self.background.reserve(CHUNK_BOUNDARY_VOXEL_COUNT);
        let last = CHUNK_SIZE as i32 - 1;

        for y in 0..=last {
            for z in 0..=last {
                self.enqueue(origin + IVec3::new(0, y, z));
                self.enqueue(origin + IVec3::new(last, y, z));
            }
        }

        for y in 0..=last {
            for x in 1..last {
                self.enqueue(origin + IVec3::new(x, y, 0));
                self.enqueue(origin + IVec3::new(x, y, last));
            }
        }

        for z in 1..last {
            for x in 1..last {
                self.enqueue(origin + IVec3::new(x, 0, z));
                self.enqueue(origin + IVec3::new(x, last, z));
            }
        }
    }

    pub fn enqueue_chunk_boundary_neighbors(&mut self, origin: IVec3) {
        self.enqueue_deferred_background_scan(
            DeferredBackgroundScanRequest::ChunkBoundaryNeighbors(origin),
        );
    }

    fn enqueue_deferred_background_scan(&mut self, request: DeferredBackgroundScanRequest) {
        let was_empty = !self.deferred_background_scans.has_work();
        if self.deferred_background_scans.enqueue(request) && was_empty {
            // Preserve the eager path's FIFO boundary without materializing
            // thousands of voxel positions. Background work that already
            // existed when the first deferred scan was requested stays ahead
            // of it; work generated afterwards stays behind deferred scans.
            self.background_items_before_deferred = self.background.len();
        }
    }

    pub fn pop(&mut self) -> Option<(IVec3, LightingLane)> {
        if let Some(position) = self.interactive.pop() {
            return Some((position, LightingLane::Interactive));
        }
        if let Some(position) = self.settling.pop() {
            return Some((position, LightingLane::Settling));
        }
        self.pop_background()
            .map(|position| (position, LightingLane::Background))
    }

    fn pop_background(&mut self) -> Option<IVec3> {
        if self.deferred_background_scans.has_active() {
            return self.pop_deferred_background();
        }

        if self.deferred_background_scans.has_work() {
            if self.background_items_before_deferred == 0 {
                return self.pop_deferred_background();
            }

            if let Some(position) = self.background.pop() {
                self.background_items_before_deferred -= 1;
                return Some(position);
            }

            // Higher-priority promotion can remove entries that were part of
            // the original watermark. If none remain, the virtual scan is now
            // the oldest background work regardless of the counter.
            self.background_items_before_deferred = 0;
            return self.pop_deferred_background();
        }

        self.background_items_before_deferred = 0;
        self.background.pop()
    }

    fn pop_deferred_background(&mut self) -> Option<IVec3> {
        let position = self.deferred_background_scans.pop()?;
        // A voxel explicitly queued after a virtual scan request would have
        // been deduplicated by the eager expansion. Preserve that behavior
        // once the deferred scan reaches the same position.
        self.background.remove(position);
        Some(position)
    }

    pub(super) fn has_interactive_work(&self) -> bool {
        self.interactive.len() > 0
    }

    pub(super) fn has_settling_work(&self) -> bool {
        self.settling.len() > 0
    }

    fn has_background_work(&self) -> bool {
        self.background.len() > 0 || self.deferred_background_scans.has_work()
    }

    pub(super) fn next_lane(&self) -> Option<LightingLane> {
        if self.has_interactive_work() {
            Some(LightingLane::Interactive)
        } else if self.has_settling_work() {
            Some(LightingLane::Settling)
        } else if self.has_background_work() {
            Some(LightingLane::Background)
        } else {
            None
        }
    }

    pub(super) fn has_work_in_lane(&self, lane: LightingLane) -> bool {
        match lane {
            LightingLane::Interactive => self.has_interactive_work(),
            LightingLane::Settling => self.has_settling_work(),
            LightingLane::Background => self.has_background_work(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.interactive.len() == 0 && self.settling.len() == 0 && !self.has_background_work()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn chunk_voxel_scan_is_lazy_and_produces_the_full_chunk() {
        let mut queue = LightingQueue::default();
        let origin = IVec3::new(32, 16, -16);
        queue.enqueue_chunk_voxels(origin);

        assert_eq!(queue.background.len(), 0);
        assert!(queue.deferred_background_scans.has_work());

        let mut count = 0;
        let mut first = None;
        let mut last = None;
        while let Some((position, lane)) = queue.pop() {
            assert_eq!(lane, LightingLane::Background);
            if first.is_none() {
                first = Some(position);
            }
            last = Some(position);
            count += 1;
        }

        assert_eq!(count, CHUNK_VOLUME);
        assert_eq!(first, Some(origin));
        assert_eq!(last, Some(origin + IVec3::splat(CHUNK_SIZE as i32 - 1)),);
    }

    #[test]
    fn deferred_chunk_scan_preserves_preexisting_background_order() {
        let mut queue = LightingQueue::default();
        let before = IVec3::new(80, 4, 80);
        let origin = IVec3::new(32, 16, -16);
        let after = IVec3::new(81, 4, 80);
        queue.enqueue(before);
        queue.enqueue_chunk_voxels(origin);
        queue.enqueue(after);

        assert_eq!(queue.pop(), Some((before, LightingLane::Background)));
        assert_eq!(queue.pop(), Some((origin, LightingLane::Background)));
        assert!(queue.background.contains(after));
    }

    #[test]
    fn deferred_chunk_requests_deduplicate_before_scanning() {
        let mut queue = LightingQueue::default();
        let origin = IVec3::new(32, 16, -16);
        queue.enqueue_chunk_voxels(origin);
        queue.enqueue_chunk_voxels(origin);

        let mut count = 0;
        while queue.pop().is_some() {
            count += 1;
        }

        assert_eq!(count, CHUNK_VOLUME);
    }

    #[test]
    fn deferred_chunk_request_during_active_scan_schedules_another_pass() {
        let mut queue = LightingQueue::default();
        let origin = IVec3::new(32, 16, -16);
        queue.enqueue_chunk_voxels(origin);
        assert_eq!(queue.pop(), Some((origin, LightingLane::Background)));

        queue.enqueue_chunk_voxels(origin);
        let mut count = 1;
        while queue.pop().is_some() {
            count += 1;
        }

        assert_eq!(count, CHUNK_VOLUME * 2);
    }

    #[test]
    fn chunk_boundary_neighbor_scan_is_lazy_and_produces_six_faces() {
        let mut queue = LightingQueue::default();
        let origin = IVec3::new(32, 16, -16);
        queue.enqueue_chunk_boundary_neighbors(origin);

        assert_eq!(queue.background.len(), 0);
        assert!(queue.deferred_background_scans.has_work());

        let mut positions = HashSet::new();
        let mut first = None;
        let mut last = None;
        while let Some((position, lane)) = queue.pop() {
            assert_eq!(lane, LightingLane::Background);
            if first.is_none() {
                first = Some(position);
            }
            last = Some(position);
            assert!(positions.insert(position));
        }

        assert_eq!(positions.len(), CHUNK_BOUNDARY_NEIGHBOR_COUNT);
        assert_eq!(first, Some(origin + IVec3::new(-1, 0, 0)));
        let last_coord = CHUNK_SIZE as i32 - 1;
        assert_eq!(
            last,
            Some(origin + IVec3::new(last_coord, CHUNK_SIZE as i32, last_coord)),
        );
    }

    #[test]
    fn chunk_boundary_neighbor_scan_filters_negative_world_y_lazily() {
        let mut queue = LightingQueue::default();
        queue.enqueue_chunk_boundary_neighbors(IVec3::ZERO);

        let mut positions = HashSet::new();
        while let Some((position, lane)) = queue.pop() {
            assert_eq!(lane, LightingLane::Background);
            assert!(position.y >= 0);
            assert!(positions.insert(position));
        }

        let face_area = CHUNK_SIZE * CHUNK_SIZE;
        assert_eq!(positions.len(), CHUNK_BOUNDARY_NEIGHBOR_COUNT - face_area);
    }

    #[test]
    fn deferred_boundary_requests_deduplicate_before_scanning() {
        let mut queue = LightingQueue::default();
        let origin = IVec3::new(32, 16, -16);
        queue.enqueue_chunk_boundary_neighbors(origin);
        queue.enqueue_chunk_boundary_neighbors(origin);

        let mut count = 0;
        while queue.pop().is_some() {
            count += 1;
        }

        assert_eq!(count, CHUNK_BOUNDARY_NEIGHBOR_COUNT);
    }

    #[test]
    fn boundary_voxels_enqueue_only_chunk_shell() {
        let mut queue = LightingQueue::default();
        queue.enqueue_chunk_boundary_voxels(IVec3::ZERO);

        let mut count = 0;
        while queue.pop().is_some() {
            count += 1;
        }

        let inner = (CHUNK_SIZE - 2).pow(3);
        assert_eq!(count, CHUNK_SIZE.pow(3) - inner);
    }

    #[test]
    fn interactive_work_preempts_background_and_promotes_duplicates() {
        let mut queue = LightingQueue::default();
        let background = IVec3::new(20, 4, 20);
        let edited = IVec3::new(3, 4, 7);
        queue.enqueue(background);
        queue.enqueue(edited);
        queue.enqueue_with_neighbors_priority(edited);

        assert_eq!(queue.pop(), Some((edited, LightingLane::Interactive)));
        assert!(queue.has_interactive_work());

        while let Some((_, lane)) = queue.pop() {
            if lane == LightingLane::Background {
                break;
            }
        }
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn propagated_interactive_work_stays_in_interactive_lane() {
        let mut queue = LightingQueue::default();
        let position = IVec3::new(3, 4, 7);
        queue.enqueue(IVec3::new(20, 4, 20));
        queue.enqueue_with_neighbors_priority(position);
        let (_, lane) = queue.pop().expect("interactive seed should be queued");

        queue.enqueue_with_neighbors_in_lane(position, lane);

        assert_eq!(
            queue.pop().map(|(_, lane)| lane),
            Some(LightingLane::Interactive)
        );
    }

    #[test]
    fn settling_work_preempts_background_and_preserves_its_lane() {
        let mut queue = LightingQueue::default();
        let background = IVec3::new(20, 4, 20);
        let settling = IVec3::new(3, 4, 7);
        queue.enqueue(background);
        queue.enqueue_settling(settling);

        assert_eq!(queue.pop(), Some((settling, LightingLane::Settling)));
        queue.enqueue_with_neighbors_in_lane(settling, LightingLane::Settling);

        assert_eq!(
            queue.pop().map(|(_, lane)| lane),
            Some(LightingLane::Settling)
        );
        assert!(queue.has_settling_work());
    }
}
