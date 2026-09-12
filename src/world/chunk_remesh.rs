use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::voxel::{
    deduplicated_queue::DeduplicatedQueue, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
};

use super::{
    chunk_rendering::refresh_chunk_mesh,
    chunk_system_params::{ChunkContent, ChunkRenderer},
};

const REMESH_BUDGET: Duration = Duration::from_millis(4);

#[derive(Resource, Default)]
pub(crate) struct ChunkRemeshQueue {
    queue: DeduplicatedQueue<IVec3>,
}

impl ChunkRemeshQueue {
    pub(crate) fn enqueue(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.queue.enqueue(coord);
        }
    }

    pub(crate) fn enqueue_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.queue.enqueue_front(coord);
        }
    }

    pub(crate) fn enqueue_voxel_edit(&mut self, coord: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_priority(coord + offset);
        }
        self.enqueue_priority(coord);
    }

    pub(crate) fn enqueue_voxel_edit_neighbors(&mut self, coord: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_priority(coord + offset);
        }
    }

    pub(crate) fn extend(&mut self, coords: impl IntoIterator<Item = IVec3>) {
        for coord in coords {
            self.enqueue(coord);
        }
    }

    pub(crate) fn extend_priority(&mut self, coords: impl IntoIterator<Item = IVec3>) {
        for coord in coords {
            self.enqueue_priority(coord);
        }
    }

    fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    fn clear(&mut self) {
        self.queue.clear();
    }
}

pub(super) fn clear_chunk_remesh_queue(mut queue: ResMut<ChunkRemeshQueue>) {
    queue.clear();
}

pub(super) fn process_chunk_remesh_queue(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
) {
    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );
    let frame_started = Instant::now();
    let mut processed = 0;

    loop {
        if processed > 0 && frame_started.elapsed() >= REMESH_BUDGET {
            break;
        }

        let Some(coord) = queue.pop() else {
            break;
        };

        refresh_chunk_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            &render_context,
        );
        processed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_deduplicates_chunks() {
        let mut queue = ChunkRemeshQueue::default();
        queue.enqueue(IVec3::X);
        queue.enqueue(IVec3::X);

        assert_eq!(queue.pop(), Some(IVec3::X));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn priority_enqueue_moves_existing_chunk_to_front() {
        let mut queue = ChunkRemeshQueue::default();
        queue.enqueue(IVec3::X);
        queue.enqueue(IVec3::Z);
        queue.enqueue_priority(IVec3::Z);

        assert_eq!(queue.pop(), Some(IVec3::Z));
        assert_eq!(queue.pop(), Some(IVec3::X));
    }

    #[test]
    fn voxel_edit_prioritizes_edited_chunk_before_neighbors() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(4, 2, -3);
        queue.enqueue_voxel_edit(coord);

        assert_eq!(queue.pop(), Some(coord));
    }
}
