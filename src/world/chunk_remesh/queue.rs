use bevy::prelude::*;

use crate::{
    voxel::{
        deduplicated_queue::DeduplicatedQueue,
        neighbors::CARDINAL_NEIGHBORS,
        world::VoxelWorld,
    },
    world::{
        chunk_remesh_tasks::ChunkRemeshTaskKind,
        chunk_rendering::ChunkRenderPool,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RenderableScanKey {
    queue_revision: u64,
    pool_revision: u64,
}

#[derive(Resource, Default)]
pub(crate) struct ChunkRemeshQueue {
    queue: DeduplicatedQueue<IVec3>,
    fluid: DeduplicatedQueue<IVec3>,
    lighting: DeduplicatedQueue<IVec3>,
    geometry_scan_miss: Option<RenderableScanKey>,
    fluid_scan_miss: Option<RenderableScanKey>,
    lighting_scan_miss: Option<RenderableScanKey>,
    next_background_kind: usize,
}

impl ChunkRemeshQueue {
    #[cfg(test)]
    pub(crate) fn enqueue(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.queue.enqueue(coord);
        }
    }

    pub(crate) fn enqueue_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            // Terrain-only remesh cannot satisfy an independent fluid remesh.
            self.queue.enqueue_front(coord);
        }
    }

    pub(crate) fn enqueue_fluid(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.enqueue(coord);
        }
    }

    pub(crate) fn enqueue_fluid_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.enqueue_front(coord);
        }
    }

    fn enqueue_lighting_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.lighting.enqueue_front(coord);
        }
    }

    pub(super) fn enqueue_task_priority(&mut self, coord: IVec3, kind: ChunkRemeshTaskKind) {
        match kind {
            ChunkRemeshTaskKind::Geometry => self.enqueue_priority(coord),
            ChunkRemeshTaskKind::Lighting => self.enqueue_lighting_priority(coord),
            ChunkRemeshTaskKind::Fluid => self.enqueue_fluid_priority(coord),
        }
    }

    pub(crate) fn enqueue_voxel_edit(&mut self, coord: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_priority(coord + offset);
        }

        if coord.y >= 0 {
            self.enqueue_priority(coord);
        }
    }

    pub(crate) fn enqueue_lighting_change(&mut self, coord: IVec3, world: &VoxelWorld) {
        self.enqueue_lighting_priority(coord);
        if world.chunk(coord).is_some_and(|chunk| chunk.has_fluid()) {
            self.enqueue_fluid_priority(coord);
        }

        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if neighbor.y >= 0 {
                self.lighting.enqueue(neighbor);
                // Fluid vertices bake face lighting from the one-voxel halo too.
                // Rebuild only chunks with actual fluid, using occupancy metadata.
                if world.chunk(neighbor).is_some_and(|chunk| chunk.has_fluid()) {
                    self.fluid.enqueue(neighbor);
                }
            }
        }
    }

    pub(crate) fn remove(&mut self, coord: IVec3) {
        self.queue.remove(coord);
        self.fluid.remove(coord);
        self.lighting.remove(coord);
    }

    pub(super) fn has_background_work(&self) -> bool {
        self.queue.len() > 0 || self.fluid.len() > 0 || self.lighting.len() > 0
    }

    fn pop_renderable_geometry(&mut self, render_pool: &ChunkRenderPool) -> Option<IVec3> {
        pop_renderable_from(
            &mut self.queue,
            &mut self.geometry_scan_miss,
            render_pool,
        )
    }

    fn pop_renderable_fluid(&mut self, render_pool: &ChunkRenderPool) -> Option<IVec3> {
        pop_renderable_from(&mut self.fluid, &mut self.fluid_scan_miss, render_pool)
    }

    fn coalesce_geometry_into_lighting(&mut self, coord: IVec3) {
        // Geometry and Lighting rebuild the same terrain mesh, but neither
        // rebuilds fluid meshes. Keep the fluid request independent.
        self.queue.remove(coord);
    }

    fn pop_renderable_lighting(&mut self, render_pool: &ChunkRenderPool) -> Option<IVec3> {
        let coord = pop_renderable_from(
            &mut self.lighting,
            &mut self.lighting_scan_miss,
            render_pool,
        )?;
        self.coalesce_geometry_into_lighting(coord);
        Some(coord)
    }

    pub(super) fn pop_renderable_background(
        &mut self,
        render_pool: &ChunkRenderPool,
        allow_terrain: bool,
        allow_fluid: bool,
    ) -> Option<(IVec3, ChunkRemeshTaskKind)> {
        const KIND_COUNT: usize = 3;

        for offset in 0..KIND_COUNT {
            let kind_index = (self.next_background_kind + offset) % KIND_COUNT;
            let next = match kind_index {
                0 if allow_fluid => self
                    .pop_renderable_fluid(render_pool)
                    .map(|coord| (coord, ChunkRemeshTaskKind::Fluid)),
                1 if allow_terrain => self
                    .pop_renderable_lighting(render_pool)
                    .map(|coord| (coord, ChunkRemeshTaskKind::Lighting)),
                2 if allow_terrain => self
                    .pop_renderable_geometry(render_pool)
                    .map(|coord| (coord, ChunkRemeshTaskKind::Geometry)),
                0..=2 => None,
                _ => unreachable!("background remesh kind index must stay in range"),
            };

            if next.is_some() {
                self.next_background_kind = (kind_index + 1) % KIND_COUNT;
                return next;
            }
        }

        None
    }

    #[cfg(test)]
    fn pop(&mut self) -> Option<IVec3> {
        self.queue.pop()
    }

    #[cfg(test)]
    fn pop_fluid(&mut self) -> Option<IVec3> {
        self.fluid.pop()
    }

    #[cfg(test)]
    fn pop_lighting(&mut self) -> Option<IVec3> {
        self.lighting.pop()
    }
}

fn pop_renderable_from(
    queue: &mut DeduplicatedQueue<IVec3>,
    last_miss: &mut Option<RenderableScanKey>,
    render_pool: &ChunkRenderPool,
) -> Option<IVec3> {
    let scan_key = RenderableScanKey {
        queue_revision: queue.revision(),
        pool_revision: render_pool.membership_revision(),
    };
    if *last_miss == Some(scan_key) {
        return None;
    }

    let coord = queue.pop_where(|coord| render_pool.contains(coord));
    if coord.is_some() {
        *last_miss = None;
    } else {
        *last_miss = Some(scan_key);
    }
    coord
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{chunk::VoxelChunk, fluid::FluidCell};

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
    fn terrain_remesh_never_discards_pending_fluid_remesh() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_fluid_priority(coord);
        queue.enqueue_priority(coord);

        assert_eq!(queue.pop(), Some(coord));
        assert_eq!(queue.pop_fluid(), Some(coord));
    }

    #[test]
    fn lighting_remesh_coalesces_terrain_but_preserves_fluid() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_priority(coord);
        queue.enqueue_fluid_priority(coord);
        queue.enqueue_lighting_priority(coord);

        queue.coalesce_geometry_into_lighting(coord);

        assert_eq!(queue.pop(), None);
        assert_eq!(queue.pop_lighting(), Some(coord));
        assert_eq!(queue.pop_fluid(), Some(coord));
    }

    #[test]
    fn lighting_change_refreshes_only_chunks_containing_fluid() {
        let coord = IVec3::new(2, 1, 3);
        let neighbor = coord + IVec3::X;
        let mut world = VoxelWorld::default();
        let mut fluid_chunk = VoxelChunk::empty();
        fluid_chunk.set_fluid(8, 8, 8, Some(FluidCell::source(0, 8)));
        world.insert_chunk(coord, fluid_chunk);
        world.insert_chunk(neighbor, VoxelChunk::empty());
        let mut queue = ChunkRemeshQueue::default();

        queue.enqueue_lighting_change(neighbor, &world);

        assert_eq!(queue.pop_fluid(), Some(coord));
        assert_eq!(queue.pop_fluid(), None);
        assert_eq!(queue.pop_lighting(), Some(neighbor));
    }

    #[test]
    fn voxel_edit_keeps_lighting_refresh_separate_from_geometry() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(4, 2, -3);
        queue.enqueue_voxel_edit(coord);

        assert_eq!(queue.pop_lighting(), None);
        assert_eq!(queue.pop(), Some(coord));

        let mut queued = Vec::new();
        while let Some(value) = queue.pop() {
            queued.push(value);
        }
        for offset in CARDINAL_NEIGHBORS {
            assert!(queued.contains(&(coord + offset)));
        }
    }

    #[test]
    fn lighting_change_uses_background_lighting_remesh_queue() {
        let mut queue = ChunkRemeshQueue::default();
        let world = VoxelWorld::default();
        let coord = IVec3::new(4, 2, -3);
        queue.enqueue_lighting_change(coord, &world);

        let mut lighting = Vec::new();
        while let Some(value) = queue.pop_lighting() {
            lighting.push(value);
        }

        assert!(lighting.contains(&coord));
        for offset in CARDINAL_NEIGHBORS {
            assert!(lighting.contains(&(coord + offset)));
        }
        assert_eq!(queue.pop(), None);
        assert_eq!(queue.pop_fluid(), None);
    }

    #[test]
    fn removal_clears_all_pending_remesh_kinds() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_priority(coord);
        queue.enqueue_fluid_priority(coord);
        queue.lighting.enqueue(coord);

        queue.remove(coord);

        assert_eq!(queue.pop(), None);
        assert_eq!(queue.pop_fluid(), None);
        assert_eq!(queue.pop_lighting(), None);
    }

    #[test]
    fn geometry_renderable_scan_miss_retries_only_after_queue_change() {
        let mut queue = ChunkRemeshQueue::default();
        let render_pool = ChunkRenderPool::default();
        let first = IVec3::new(1, 1, 1);
        let second = IVec3::new(2, 1, 2);
        queue.enqueue_priority(first);

        assert_eq!(queue.pop_renderable_geometry(&render_pool), None);
        let first_miss = queue.geometry_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(queue.pop_renderable_geometry(&render_pool), None);
        assert_eq!(queue.geometry_scan_miss, first_miss);

        queue.enqueue_priority(second);
        assert_eq!(queue.pop_renderable_geometry(&render_pool), None);
        assert_ne!(queue.geometry_scan_miss, first_miss);
    }

    #[test]
    fn lighting_renderable_scan_miss_retries_only_after_queue_change() {
        let mut queue = ChunkRemeshQueue::default();
        let render_pool = ChunkRenderPool::default();
        let first = IVec3::new(1, 1, 1);
        let second = IVec3::new(2, 1, 2);
        queue.enqueue_lighting_priority(first);

        assert_eq!(queue.pop_renderable_lighting(&render_pool), None);
        let first_miss = queue.lighting_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(queue.pop_renderable_lighting(&render_pool), None);
        assert_eq!(queue.lighting_scan_miss, first_miss);

        queue.enqueue_lighting_priority(second);
        assert_eq!(queue.pop_renderable_lighting(&render_pool), None);
        assert_ne!(queue.lighting_scan_miss, first_miss);
    }

    #[test]
    fn fluid_renderable_scan_miss_retries_only_after_queue_change() {
        let mut queue = ChunkRemeshQueue::default();
        let render_pool = ChunkRenderPool::default();
        let first = IVec3::new(1, 1, 1);
        let second = IVec3::new(2, 1, 2);
        queue.enqueue_fluid_priority(first);

        assert_eq!(queue.pop_renderable_fluid(&render_pool), None);
        let first_miss = queue.fluid_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(queue.pop_renderable_fluid(&render_pool), None);
        assert_eq!(queue.fluid_scan_miss, first_miss);

        queue.enqueue_fluid_priority(second);
        assert_eq!(queue.pop_renderable_fluid(&render_pool), None);
        assert_ne!(queue.fluid_scan_miss, first_miss);
    }
}
