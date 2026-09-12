use bevy::prelude::*;

use super::{deduplicated_queue::DeduplicatedQueue, neighbors::CARDINAL_NEIGHBORS};

#[derive(Debug, Default)]
pub(crate) struct VoxelUpdateQueue {
    queue: DeduplicatedQueue<IVec3>,
}

impl VoxelUpdateQueue {
    pub(crate) fn enqueue(&mut self, position: IVec3) -> bool {
        if position.y < 0 {
            return false;
        }

        self.queue.enqueue(position)
    }

    pub(crate) fn enqueue_priority(&mut self, position: IVec3) {
        if position.y < 0 {
            return;
        }

        self.queue.enqueue_front(position);
    }

    pub(crate) fn enqueue_with_neighbors(&mut self, position: IVec3) {
        self.enqueue(position);
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue(position + offset);
        }
    }

    pub(crate) fn enqueue_with_neighbors_priority(&mut self, position: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_priority(position + offset);
        }
        self.enqueue_priority(position);
    }

    pub(crate) fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    pub(crate) fn len(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn clear(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_enqueue_includes_nonnegative_neighbors_once() {
        let mut queue = VoxelUpdateQueue::default();
        queue.enqueue_with_neighbors(IVec3::ZERO);
        queue.enqueue_with_neighbors(IVec3::ZERO);

        assert_eq!(queue.len(), 6);
        let mut positions = (0..queue.len())
            .filter_map(|_| queue.pop())
            .collect::<Vec<_>>();
        positions.sort_by_key(|position| (position.x, position.y, position.z));

        assert!(positions.contains(&IVec3::ZERO));
        assert!(!positions.contains(&IVec3::NEG_Y));
    }

    #[test]
    fn priority_edit_puts_the_edited_voxel_first() {
        let mut queue = VoxelUpdateQueue::default();
        queue.enqueue(IVec3::new(20, 4, 20));
        queue.enqueue_with_neighbors_priority(IVec3::new(3, 4, 7));

        assert_eq!(queue.pop(), Some(IVec3::new(3, 4, 7)));
    }
}
