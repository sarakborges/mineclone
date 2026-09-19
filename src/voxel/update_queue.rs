use bevy::prelude::*;

use super::{deduplicated_queue::DeduplicatedQueue, neighbors::CARDINAL_NEIGHBORS};

#[derive(Debug, Default)]
pub(crate) struct VoxelUpdateQueue {
    queue: DeduplicatedQueue<IVec3>,
}

impl VoxelUpdateQueue {
    pub(crate) fn reserve(&mut self, additional: usize) {
        self.queue.reserve(additional);
    }

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

    pub(crate) fn enqueue_with_neighbors_priority(&mut self, position: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_priority(position + offset);
        }
        self.enqueue_priority(position);
    }

    pub(crate) fn contains(&self, position: IVec3) -> bool {
        self.queue.contains(position)
    }

    pub(crate) fn remove(&mut self, position: IVec3) -> bool {
        self.queue.remove(position)
    }

    pub(crate) fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    pub(crate) fn len(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.queue.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_edit_puts_the_edited_voxel_first() {
        let mut queue = VoxelUpdateQueue::default();
        queue.enqueue(IVec3::new(20, 4, 20));
        queue.enqueue_with_neighbors_priority(IVec3::new(3, 4, 7));

        assert_eq!(queue.pop(), Some(IVec3::new(3, 4, 7)));
    }

    #[test]
    fn remove_clears_membership() {
        let position = IVec3::new(3, 4, 7);
        let mut queue = VoxelUpdateQueue::default();
        queue.enqueue(position);

        assert!(queue.contains(position));
        assert!(queue.remove(position));
        assert!(!queue.contains(position));
        assert_eq!(queue.pop(), None);
    }
}
