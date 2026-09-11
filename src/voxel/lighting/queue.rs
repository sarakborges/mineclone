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
}
