use bevy::{platform::collections::HashMap, prelude::*};

use crate::voxel::{
    chunk::{CHUNK_SIZE, CHUNK_VOLUME},
    coordinates::chunk_coord_from_world,
    neighbors::CARDINAL_NEIGHBORS,
    update_queue::VoxelUpdateQueue,
};

const CHUNK_INTERIOR_VOLUME: usize =
    (CHUNK_SIZE - 2) * (CHUNK_SIZE - 2) * (CHUNK_SIZE - 2);
const CHUNK_BOUNDARY_VOXEL_COUNT: usize = CHUNK_VOLUME - CHUNK_INTERIOR_VOLUME;
const CHUNK_BOUNDARY_NEIGHBOR_COUNT: usize = 6 * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LightingLane {
    Interactive,
    Background,
}

#[derive(Default)]
pub(super) struct LightingQueue {
    interactive: VoxelUpdateQueue,
    background: VoxelUpdateQueue,
    pending_by_chunk: HashMap<IVec3, usize>,
}

impl LightingQueue {
    pub fn enqueue(&mut self, position: IVec3) {
        if self.interactive.contains(position) {
            return;
        }
        if self.background.enqueue(position) {
            self.track_new_position(position);
        }
    }

    pub(super) fn enqueue_interactive(&mut self, position: IVec3) {
        let promoted = self.background.remove(position);
        if self.interactive.enqueue(position) && !promoted {
            self.track_new_position(position);
        }
    }

    fn enqueue_interactive_priority(&mut self, position: IVec3) {
        let promoted = self.background.remove(position);
        let already_interactive = self.interactive.contains(position);
        self.interactive.enqueue_priority(position);
        if !promoted && !already_interactive && position.y >= 0 {
            self.track_new_position(position);
        }
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

    pub(super) fn enqueue_with_neighbors_in_lane(
        &mut self,
        position: IVec3,
        lane: LightingLane,
    ) {
        match lane {
            LightingLane::Interactive => {
                self.enqueue_interactive(position);
                for offset in CARDINAL_NEIGHBORS {
                    self.enqueue_interactive(position + offset);
                }
            }
            LightingLane::Background => self.enqueue_with_neighbors(position),
        }
    }

    pub fn enqueue_chunk_voxels(&mut self, origin: IVec3) {
        self.background.reserve(CHUNK_VOLUME);
        let size = CHUNK_SIZE as i32;

        for y in 0..size {
            for z in 0..size {
                for x in 0..size {
                    self.enqueue(origin + IVec3::new(x, y, z));
                }
            }
        }
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
        self.background.reserve(CHUNK_BOUNDARY_NEIGHBOR_COUNT);
        let size = CHUNK_SIZE as i32;

        for y in 0..size {
            for z in 0..size {
                self.enqueue(origin + IVec3::new(-1, y, z));
                self.enqueue(origin + IVec3::new(size, y, z));
            }
        }

        for y in 0..size {
            for x in 0..size {
                self.enqueue(origin + IVec3::new(x, y, -1));
                self.enqueue(origin + IVec3::new(x, y, size));
            }
        }

        for z in 0..size {
            for x in 0..size {
                self.enqueue(origin + IVec3::new(x, -1, z));
                self.enqueue(origin + IVec3::new(x, size, z));
            }
        }
    }

    pub fn pop(&mut self) -> Option<(IVec3, LightingLane)> {
        if let Some(position) = self.interactive.pop() {
            self.track_popped_position(position);
            return Some((position, LightingLane::Interactive));
        }
        let position = self.background.pop()?;
        self.track_popped_position(position);
        Some((position, LightingLane::Background))
    }

    pub(super) fn has_interactive_work(&self) -> bool {
        self.interactive.len() > 0
    }

    pub fn is_empty(&self) -> bool {
        self.interactive.len() == 0 && self.background.len() == 0
    }

    pub(super) fn has_pending_in_chunk(&self, coord: IVec3) -> bool {
        self.pending_by_chunk.get(&coord).copied().unwrap_or(0) > 0
    }

    pub(super) fn has_pending_in_halo(&self, coord: IVec3) -> bool {
        for y in -1..=1 {
            for z in -1..=1 {
                for x in -1..=1 {
                    if self.has_pending_in_chunk(coord + IVec3::new(x, y, z)) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn track_new_position(&mut self, position: IVec3) {
        if position.y < 0 {
            return;
        }
        let coord = chunk_coord_from_world(position);
        *self.pending_by_chunk.entry(coord).or_insert(0) += 1;
    }

    fn track_popped_position(&mut self, position: IVec3) {
        if position.y < 0 {
            return;
        }
        let coord = chunk_coord_from_world(position);
        let Some(count) = self.pending_by_chunk.get_mut(&coord) else {
            debug_assert!(false, "popped lighting position must be tracked by chunk");
            return;
        };
        *count = count
            .checked_sub(1)
            .expect("lighting pending chunk count cannot underflow");
        if *count == 0 {
            self.pending_by_chunk.remove(&coord);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_chunk_counts_follow_enqueue_promotion_and_pop() {
        let mut queue = LightingQueue::default();
        let position = IVec3::new(CHUNK_SIZE as i32, 3, 4);
        let coord = IVec3::X;

        queue.enqueue(position);
        assert!(queue.has_pending_in_chunk(coord));
        queue.enqueue_interactive(position);
        assert!(queue.has_pending_in_chunk(coord));

        assert_eq!(queue.pop(), Some((position, LightingLane::Interactive)));
        assert!(!queue.has_pending_in_chunk(coord));
    }

    #[test]
    fn pending_halo_detects_neighbor_section_work() {
        let mut queue = LightingQueue::default();
        let center = IVec3::new(4, 2, 7);
        let neighbor = center + IVec3::X;
        let position = neighbor * CHUNK_SIZE as i32 + IVec3::new(2, 3, 4);

        queue.enqueue(position);

        assert!(queue.has_pending_in_halo(center));
        while queue.pop().is_some() {}
        assert!(!queue.has_pending_in_halo(center));
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

        assert_eq!(queue.pop().map(|(_, lane)| lane), Some(LightingLane::Interactive));
    }
}
