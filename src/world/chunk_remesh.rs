use std::time::Duration;

use bevy::prelude::*;

use crate::voxel::{
    deduplicated_queue::DeduplicatedQueue, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
};

use super::{
    chunk_rendering::{ChunkRenderPool, refresh_chunk_fluid_mesh, refresh_chunk_geometry_mesh},
    chunk_system_params::{ChunkContent, ChunkRenderer},
    work_budget::FrameWorkBudget,
};

const REMESH_BUDGET: Duration = Duration::from_millis(1);
const MAX_IMMEDIATE_LIGHTING_REMESHES_PER_FRAME: usize = 2;

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

    pub(crate) fn remove(&mut self, coord: IVec3) {
        self.queue.remove(coord);
        self.fluid.remove(coord);
        self.immediate_geometry.remove(coord);
        self.immediate_lighting.remove(coord);
    }

    fn pop_renderable(&mut self, render_pool: &ChunkRenderPool) -> Option<IVec3> {
        let coord = self.queue.pop_where(|coord| render_pool.contains(coord))?;
        self.fluid.remove(coord);
        Some(coord)
    }

    fn pop_renderable_fluid(&mut self, render_pool: &ChunkRenderPool) -> Option<IVec3> {
        self.fluid.pop_where(|coord| render_pool.contains(coord))
    }

    fn pop_renderable_immediate_geometry(
        &mut self,
        render_pool: &ChunkRenderPool,
    ) -> Option<IVec3> {
        let coord = self
            .immediate_geometry
            .pop_where(|coord| render_pool.contains(coord))?;
        self.queue.remove(coord);
        self.fluid.remove(coord);
        Some(coord)
    }

    fn pop_renderable_immediate_lighting(
        &mut self,
        render_pool: &ChunkRenderPool,
    ) -> Option<IVec3> {
        self.immediate_lighting
            .pop_where(|coord| render_pool.contains(coord))
    }

    #[cfg(test)]
    fn pop(&mut self) -> Option<IVec3> {
        let coord = self.queue.pop()?;
        self.fluid.remove(coord);
        Some(coord)
    }

    #[cfg(test)]
    fn pop_fluid(&mut self) -> Option<IVec3> {
        self.fluid.pop()
    }

    #[cfg(test)]
    fn pop_immediate_geometry(&mut self) -> Option<IVec3> {
        let coord = self.immediate_geometry.pop()?;
        self.queue.remove(coord);
        self.fluid.remove(coord);
        Some(coord)
    }

    #[cfg(test)]
    fn pop_immediate_lighting(&mut self) -> Option<IVec3> {
        self.immediate_lighting.pop()
    }
}

pub(super) fn process_immediate_geometry_remesh(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
) {
    let Some(coord) = queue.pop_renderable_immediate_geometry(&renderer.pool) else {
        return;
    };
    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );

    refresh_chunk_geometry_mesh(
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
    let Some(first_coord) = queue.pop_renderable_immediate_lighting(&renderer.pool) else {
        return;
    };
    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );

    refresh_chunk_geometry_mesh(
        &mut renderer.commands,
        &mut renderer.meshes,
        &mut renderer.pool,
        first_coord,
        &render_context,
    );

    for _ in 1..MAX_IMMEDIATE_LIGHTING_REMESHES_PER_FRAME {
        let Some(coord) = queue.pop_renderable_immediate_lighting(&renderer.pool) else {
            break;
        };

        refresh_chunk_geometry_mesh(
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
    let first = if let Some(coord) = queue.pop_renderable(&renderer.pool) {
        Some((coord, true))
    } else {
        queue
            .pop_renderable_fluid(&renderer.pool)
            .map(|coord| (coord, false))
    };
    let Some((first_coord, first_is_geometry)) = first else {
        return;
    };

    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );
    let mut budget = FrameWorkBudget::new(REMESH_BUDGET, 1);

    if first_is_geometry {
        refresh_chunk_geometry_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            first_coord,
            &render_context,
        );
    } else {
        refresh_chunk_fluid_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            first_coord,
            &render_context,
        );
    }
    budget.record(1);

    loop {
        if budget.exhausted() {
            break;
        }

        if let Some(coord) = queue.pop_renderable(&renderer.pool) {
            refresh_chunk_geometry_mesh(
                &mut renderer.commands,
                &mut renderer.meshes,
                &mut renderer.pool,
                coord,
                &render_context,
            );
            budget.record(1);
            continue;
        }

        let Some(coord) = queue.pop_renderable_fluid(&renderer.pool) else {
            break;
        };

        refresh_chunk_fluid_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            &render_context,
        );
        budget.record(1);
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

    #[test]
    fn removal_clears_all_pending_remesh_kinds() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_priority(coord);
        queue.enqueue_fluid_priority(coord);
        queue.immediate_geometry.enqueue(coord);
        queue.immediate_lighting.enqueue(coord);

        queue.remove(coord);

        assert_eq!(queue.pop(), None);
        assert_eq!(queue.pop_fluid(), None);
        assert_eq!(queue.pop_immediate_geometry(), None);
        assert_eq!(queue.pop_immediate_lighting(), None);
    }
}
