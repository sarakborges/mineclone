use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::voxel::{
    deduplicated_queue::DeduplicatedQueue, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
};

use super::{
    chunk_rendering::{
        refresh_chunk_fluid_mesh, refresh_chunk_lighting_mesh, refresh_chunk_mesh,
    },
    chunk_system_params::{ChunkContent, ChunkRenderer},
};

const REMESH_BUDGET: Duration = Duration::from_millis(2);
const MAX_IMMEDIATE_LIGHTING_REMESHES_PER_FRAME: usize = 6;

#[derive(Resource, Default)]
pub(crate) struct ChunkRemeshQueue {
    queue: DeduplicatedQueue<IVec3>,
    fluid: DeduplicatedQueue<IVec3>,
    immediate_geometry: DeduplicatedQueue<IVec3>,
    immediate_lighting: DeduplicatedQueue<IVec3>,
}

impl ChunkRemeshQueue {
    #[cfg(test)]
    pub(crate) fn enqueue(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.remove(coord);
            self.queue.enqueue(coord);
        }
    }

    pub(crate) fn enqueue_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.remove(coord);
            self.queue.enqueue_front(coord);
        }
    }

    pub(crate) fn enqueue_fluid_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.enqueue_front(coord);
        }
    }

    pub(crate) fn enqueue_voxel_edit(&mut self, coord: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_priority(coord + offset);
        }

        if coord.y >= 0 {
            self.enqueue_priority(coord);
            self.immediate_geometry.enqueue_front(coord);
        }
    }

    pub(crate) fn enqueue_lighting_change(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.immediate_lighting.enqueue_front(coord);
        }

        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if neighbor.y >= 0 {
                self.immediate_lighting.enqueue(neighbor);
            }
        }
    }

    fn pop(&mut self) -> Option<IVec3> {
        let coord = self.queue.pop()?;
        self.fluid.remove(coord);
        Some(coord)
    }

    fn pop_fluid(&mut self) -> Option<IVec3> {
        self.fluid.pop()
    }

    fn pop_immediate_geometry(&mut self) -> Option<IVec3> {
        let coord = self.immediate_geometry.pop()?;
        self.queue.remove(coord);
        self.fluid.remove(coord);
        Some(coord)
    }

    fn pop_immediate_lighting(&mut self) -> Option<IVec3> {
        self.immediate_lighting.pop()
    }

    fn clear(&mut self) {
        self.queue.clear();
        self.fluid.clear();
        self.immediate_geometry.clear();
        self.immediate_lighting.clear();
    }
}

pub(super) fn clear_chunk_remesh_queue(mut queue: ResMut<ChunkRemeshQueue>) {
    queue.clear();
}

pub(super) fn process_immediate_geometry_remesh(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
) {
    let Some(coord) = queue.pop_immediate_geometry() else {
        return;
    };
    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );

    refresh_chunk_mesh(
        &mut renderer.commands,
        &mut renderer.meshes,
        &mut renderer.pool,
        coord,
        &render_context,
    );
}

pub(super) fn process_immediate_lighting_remesh(
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

    for _ in 0..MAX_IMMEDIATE_LIGHTING_REMESHES_PER_FRAME {
        let Some(coord) = queue.pop_immediate_lighting() else {
            break;
        };

        refresh_chunk_lighting_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            &render_context,
        );
    }
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

        if let Some(coord) = queue.pop() {
            refresh_chunk_mesh(
                &mut renderer.commands,
                &mut renderer.meshes,
                &mut renderer.pool,
                coord,
                &render_context,
            );
            processed += 1;
            continue;
        }

        let Some(coord) = queue.pop_fluid() else {
            break;
        };

        refresh_chunk_fluid_mesh(
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
    fn full_remesh_supersedes_pending_fluid_remesh() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_fluid_priority(coord);
        queue.enqueue_priority(coord);

        assert_eq!(queue.pop(), Some(coord));
        assert_eq!(queue.pop_fluid(), None);
    }

    #[test]
    fn voxel_edit_keeps_lighting_refresh_separate_from_geometry() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(4, 2, -3);
        queue.enqueue_voxel_edit(coord);

        assert_eq!(queue.pop_immediate_geometry(), Some(coord));
        assert_eq!(queue.pop_immediate_lighting(), None);

        let mut queued = Vec::new();
        while let Some(value) = queue.pop() {
            queued.push(value);
        }
        assert!(!queued.contains(&coord));
        for offset in CARDINAL_NEIGHBORS {
            assert!(queued.contains(&(coord + offset)));
        }
    }

    #[test]
    fn lighting_change_uses_only_stable_lighting_remesh_queue() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(4, 2, -3);
        queue.enqueue_lighting_change(coord);

        let mut lighting = Vec::new();
        while let Some(value) = queue.pop_immediate_lighting() {
            lighting.push(value);
        }

        assert!(lighting.contains(&coord));
        for offset in CARDINAL_NEIGHBORS {
            assert!(lighting.contains(&(coord + offset)));
        }
        assert_eq!(queue.pop(), None);
    }
}
