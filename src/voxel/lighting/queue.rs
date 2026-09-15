use bevy::prelude::*;

use crate::voxel::{chunk::CHUNK_SIZE, update_queue::VoxelUpdateQueue};

#[derive(Default)]
pub(super) struct LightingQueue {
    queue: VoxelUpdateQueue,
}

impl LightingQueue {
    pub fn enqueue(&mut self, position: IVec3) {
        self.queue.enqueue(position);
    }

    pub fn enqueue_with_neighbors(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors(position);
    }

    pub fn enqueue_with_neighbors_priority(&mut self, position: IVec3) {
        self.queue.enqueue_with_neighbors_priority(position);
    }

    pub fn enqueue_chunk_voxels(&mut self, origin: IVec3) {
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

    pub fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
