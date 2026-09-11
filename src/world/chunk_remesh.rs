use std::{
    collections::{HashSet, VecDeque},
    time::Instant,
};

use bevy::prelude::*;

use crate::voxel::world::VoxelWorld;

use super::{
    chunk_rendering::refresh_chunk_mesh,
    chunk_system_params::{ChunkContent, ChunkRenderer},
};

const REMESH_BUDGET_MS: u128 = 4;

#[derive(Resource, Default)]
pub(crate) struct ChunkRemeshQueue {
    pending: VecDeque<IVec3>,
    queued: HashSet<IVec3>,
}

impl ChunkRemeshQueue {
    pub(crate) fn enqueue(&mut self, coord: IVec3) {
        if coord.y >= 0 && self.queued.insert(coord) {
            self.pending.push_back(coord);
        }
    }

    pub(crate) fn enqueue_priority(&mut self, coord: IVec3) {
        if coord.y < 0 {
            return;
        }

        if !self.queued.insert(coord) {
            self.pending.retain(|pending| *pending != coord);
        }

        self.pending.push_front(coord);
    }

    pub(crate) fn extend(&mut self, coords: impl IntoIterator<Item = IVec3>) {
        for coord in coords {
            self.enqueue(coord);
        }
    }

    fn pop(&mut self) -> Option<IVec3> {
        let coord = self.pending.pop_front()?;
        self.queued.remove(&coord);
        Some(coord)
    }

    fn clear(&mut self) {
        self.pending.clear();
        self.queued.clear();
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
        if processed > 0 && frame_started.elapsed().as_millis() >= REMESH_BUDGET_MS {
            break;
        }

        let Some(coord) = queue.pop() else {
            break;
        };

        if !renderer.pool.contains(coord) {
            continue;
        }

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
}
