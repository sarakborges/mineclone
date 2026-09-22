use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::*};

#[cfg(test)]
use crate::voxel::neighbors::CARDINAL_NEIGHBORS;

use crate::{
    voxel::{
        coordinates::visit_chunk_coords_whose_voxel_halo_contains,
        deduplicated_queue::DeduplicatedQueue,
        meshlet::ChunkMeshletMask,
        world::VoxelWorld,
    },
    world::{
        chunk_remesh_tasks::ChunkRemeshTaskKind,
        chunk_rendering::ChunkRenderPool,
        streaming::chunk_load_priority,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RenderableScanKey {
    queue_revision: u64,
    pool_revision: u64,
}

#[derive(Default)]
struct RenderablePriorityCache {
    queue_revision: u64,
    pool_revision: u64,
    center: Option<IVec3>,
    pending: VecDeque<IVec3>,
}

impl RenderablePriorityCache {
    fn pop_nearest(
        &mut self,
        queue: &mut DeduplicatedQueue<IVec3>,
        render_pool: &ChunkRenderPool,
        center: IVec3,
    ) -> Option<IVec3> {
        let pool_revision = render_pool.membership_revision();
        if self.queue_revision != queue.revision()
            || self.pool_revision != pool_revision
            || self.center != Some(center)
        {
            self.pending.clear();
            let mut ordered = queue
                .values_in_order()
                .filter(|coord| render_pool.contains(*coord))
                .collect::<Vec<_>>();
            ordered.sort_unstable_by_key(|coord| {
                chunk_load_priority(*coord, center, IVec2::ZERO)
            });
            self.pending.extend(ordered);
            self.queue_revision = queue.revision();
            self.pool_revision = pool_revision;
            self.center = Some(center);
        }

        while let Some(coord) = self.pending.pop_front() {
            if !queue.contains(coord) || !render_pool.contains(coord) {
                continue;
            }

            let removed = queue.remove(coord);
            debug_assert!(removed, "remesh priority cache must reference an active chunk");
            self.queue_revision = queue.revision();
            return Some(coord);
        }

        None
    }
}

#[derive(Resource, Default)]
pub(crate) struct ChunkRemeshQueue {
    queue: DeduplicatedQueue<IVec3>,
    fluid: DeduplicatedQueue<IVec3>,
    lighting: DeduplicatedQueue<IVec3>,
    geometry_meshlets: HashMap<IVec3, ChunkMeshletMask>,
    fluid_meshlets: HashMap<IVec3, ChunkMeshletMask>,
    lighting_meshlets: HashMap<IVec3, ChunkMeshletMask>,
    geometry_scan_miss: Option<RenderableScanKey>,
    fluid_scan_miss: Option<RenderableScanKey>,
    lighting_scan_miss: Option<RenderableScanKey>,
    geometry_priority: RenderablePriorityCache,
    fluid_priority: RenderablePriorityCache,
    lighting_priority: RenderablePriorityCache,
    next_background_kind: usize,
}

impl ChunkRemeshQueue {
    #[cfg(test)]
    pub(crate) fn enqueue(&mut self, coord: IVec3) {
        self.enqueue_geometry_meshlets(coord, ChunkMeshletMask::ALL, false);
    }

    fn enqueue_geometry_meshlets(
        &mut self,
        coord: IVec3,
        meshlets: ChunkMeshletMask,
        priority: bool,
    ) {
        if coord.y < 0 || meshlets.is_empty() {
            return;
        }

        let combined = self
            .geometry_meshlets
            .get(&coord)
            .copied()
            .unwrap_or_default()
            .union(meshlets);
        self.geometry_meshlets.insert(coord, combined);
        if priority {
            self.queue.enqueue_front(coord);
        } else {
            self.queue.enqueue(coord);
        }
    }

    fn enqueue_fluid_meshlets(
        &mut self,
        coord: IVec3,
        meshlets: ChunkMeshletMask,
        priority: bool,
    ) {
        if coord.y < 0 || meshlets.is_empty() {
            return;
        }
        let combined = self
            .fluid_meshlets
            .get(&coord)
            .copied()
            .unwrap_or_default()
            .union(meshlets);
        self.fluid_meshlets.insert(coord, combined);
        if priority {
            self.fluid.enqueue_front(coord);
        } else {
            self.fluid.enqueue(coord);
        }
    }

    #[cfg(test)]
    pub(crate) fn enqueue_priority(&mut self, coord: IVec3) {
        self.enqueue_geometry_meshlets(coord, ChunkMeshletMask::ALL, true);
    }

    pub(crate) fn enqueue_geometry_meshlets_priority(
        &mut self,
        coord: IVec3,
        meshlets: ChunkMeshletMask,
    ) {
        self.enqueue_geometry_meshlets(coord, meshlets, true);
    }

    pub(crate) fn enqueue_fluid_meshlets_priority(
        &mut self,
        coord: IVec3,
        meshlets: ChunkMeshletMask,
    ) {
        self.enqueue_fluid_meshlets(coord, meshlets, true);
    }

    pub(crate) fn enqueue_halo_change(
        &mut self,
        coord: IVec3,
        dependency_offset: IVec3,
        geometry: bool,
        fluid: bool,
    ) {
        let meshlets = ChunkMeshletMask::for_dependency_offset(dependency_offset);
        if geometry {
            self.enqueue_geometry_meshlets(coord, meshlets, true);
        }
        if fluid {
            self.enqueue_fluid_meshlets(coord, meshlets, true);
        }
    }

    pub(crate) fn enqueue_fluid(&mut self, coord: IVec3) {
        self.enqueue_fluid_meshlets(coord, ChunkMeshletMask::ALL, false);
    }

    pub(crate) fn enqueue_fluid_priority(&mut self, coord: IVec3) {
        self.enqueue_fluid_meshlets(coord, ChunkMeshletMask::ALL, true);
    }

    fn enqueue_lighting_meshlets(
        &mut self,
        coord: IVec3,
        meshlets: ChunkMeshletMask,
        priority: bool,
    ) {
        if coord.y < 0 || meshlets.is_empty() {
            return;
        }

        let combined = self
            .lighting_meshlets
            .get(&coord)
            .copied()
            .unwrap_or_default()
            .union(meshlets);
        self.lighting_meshlets.insert(coord, combined);
        if priority {
            self.lighting.enqueue_front(coord);
        } else {
            self.lighting.enqueue(coord);
        }
    }

    #[cfg(test)]
    fn enqueue_lighting_priority(&mut self, coord: IVec3) {
        self.enqueue_lighting_meshlets(coord, ChunkMeshletMask::ALL, true);
    }

    pub(super) fn enqueue_task_priority(&mut self, coord: IVec3, kind: ChunkRemeshTaskKind) {
        self.enqueue_task_meshlets_priority(coord, kind, ChunkMeshletMask::ALL);
    }

    pub(super) fn enqueue_task_meshlets_priority(
        &mut self,
        coord: IVec3,
        kind: ChunkRemeshTaskKind,
        meshlets: ChunkMeshletMask,
    ) {
        match kind {
            ChunkRemeshTaskKind::Geometry => {
                self.enqueue_geometry_meshlets(coord, meshlets, true)
            }
            ChunkRemeshTaskKind::Lighting => {
                self.enqueue_lighting_meshlets(coord, meshlets, true)
            }
            ChunkRemeshTaskKind::Fluid => {
                self.enqueue_fluid_meshlets(coord, meshlets, true)
            }
        }
    }

    pub(crate) fn enqueue_voxel_edit(&mut self, world_position: IVec3) {
        visit_chunk_coords_whose_voxel_halo_contains(world_position, |coord| {
            let meshlets = ChunkMeshletMask::for_world_position(coord, world_position);
            self.enqueue_geometry_meshlets(coord, meshlets, true);
        });
    }

    pub(crate) fn enqueue_fluid_voxel_edit(&mut self, world_position: IVec3) {
        visit_chunk_coords_whose_voxel_halo_contains(world_position, |coord| {
            let meshlets = ChunkMeshletMask::for_world_position(coord, world_position);
            self.enqueue_fluid_meshlets(coord, meshlets, false);
        });
    }

    pub(crate) fn enqueue_lighting_meshlet_change(
        &mut self,
        coord: IVec3,
        meshlets: ChunkMeshletMask,
        world: &VoxelWorld,
    ) {
        let Some(chunk) = world.chunk(coord) else {
            return;
        };
        if chunk.has_terrain_content() {
            self.enqueue_lighting_meshlets(coord, meshlets, true);
        }
        if chunk.has_fluid() {
            self.enqueue_fluid_meshlets(coord, meshlets, true);
        }
    }

    #[cfg(test)]
    pub(crate) fn enqueue_lighting_voxel_change(
        &mut self,
        world_position: IVec3,
        world: &VoxelWorld,
    ) {
        visit_chunk_coords_whose_voxel_halo_contains(world_position, |coord| {
            let meshlets = ChunkMeshletMask::for_world_position(coord, world_position);
            self.enqueue_lighting_meshlet_change(coord, meshlets, world);
        });
    }

    #[cfg(test)]
    pub(crate) fn enqueue_lighting_change(&mut self, coord: IVec3, world: &VoxelWorld) {
        if world
            .chunk(coord)
            .is_some_and(|chunk| chunk.has_terrain_content())
        {
            self.enqueue_lighting_priority(coord);
        }
        if world.chunk(coord).is_some_and(|chunk| chunk.has_fluid()) {
            self.enqueue_fluid_priority(coord);
        }

        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if neighbor.y < 0 {
                continue;
            }
            let Some(chunk) = world.chunk(neighbor) else {
                continue;
            };

            // A one-voxel lighting halo can only influence neighbor terrain
            // that actually touches the boundary facing the changed chunk.
            let meshlets = ChunkMeshletMask::for_dependency_offset(-offset);
            if chunk.boundary_has_content(-offset) {
                self.enqueue_lighting_meshlets(neighbor, meshlets, false);
            }
            if chunk.boundary_has_fluid(-offset) {
                self.enqueue_fluid_meshlets(neighbor, meshlets, false);
            }
        }
    }

    pub(crate) fn remove(&mut self, coord: IVec3) {
        self.queue.remove(coord);
        self.fluid.remove(coord);
        self.lighting.remove(coord);
        self.geometry_meshlets.remove(&coord);
        self.fluid_meshlets.remove(&coord);
        self.lighting_meshlets.remove(&coord);
    }

    pub(super) fn has_background_work(&self) -> bool {
        self.queue.len() > 0 || self.fluid.len() > 0 || self.lighting.len() > 0
    }

    pub(crate) fn diagnostic_counts(&self) -> (usize, usize, usize) {
        (self.queue.len(), self.lighting.len(), self.fluid.len())
    }

    fn pop_renderable_geometry(
        &mut self,
        render_pool: &ChunkRenderPool,
        center: Option<IVec3>,
    ) -> Option<(IVec3, ChunkMeshletMask)> {
        let coord = if let Some(center) = center {
            self.geometry_priority
                .pop_nearest(&mut self.queue, render_pool, center)
        } else {
            pop_renderable_from(
                &mut self.queue,
                &mut self.geometry_scan_miss,
                render_pool,
            )
        }?;
        let meshlets = self
            .geometry_meshlets
            .remove(&coord)
            .unwrap_or(ChunkMeshletMask::ALL);
        Some((coord, meshlets))
    }

    fn pop_renderable_fluid(
        &mut self,
        render_pool: &ChunkRenderPool,
        center: Option<IVec3>,
    ) -> Option<(IVec3, ChunkMeshletMask)> {
        let coord = if let Some(center) = center {
            self.fluid_priority
                .pop_nearest(&mut self.fluid, render_pool, center)
        } else {
            pop_renderable_from(&mut self.fluid, &mut self.fluid_scan_miss, render_pool)
        }?;
        let meshlets = self
            .fluid_meshlets
            .remove(&coord)
            .unwrap_or(ChunkMeshletMask::ALL);
        Some((coord, meshlets))
    }

    fn pop_renderable_lighting(
        &mut self,
        render_pool: &ChunkRenderPool,
        center: Option<IVec3>,
    ) -> Option<(IVec3, ChunkMeshletMask)> {
        let coord = if let Some(center) = center {
            self.lighting_priority
                .pop_nearest(&mut self.lighting, render_pool, center)
        } else {
            pop_renderable_from(
                &mut self.lighting,
                &mut self.lighting_scan_miss,
                render_pool,
            )
        }?;
        let meshlets = self
            .lighting_meshlets
            .remove(&coord)
            .unwrap_or(ChunkMeshletMask::ALL);
        Some((coord, meshlets))
    }

    pub(super) fn coalesce_terrain_work(
        &mut self,
        coord: IVec3,
        kind: ChunkRemeshTaskKind,
        meshlets: ChunkMeshletMask,
    ) -> (ChunkRemeshTaskKind, ChunkMeshletMask) {
        match kind {
            ChunkRemeshTaskKind::Geometry => {
                let lighting = if self.lighting.remove(coord) {
                    self.lighting_meshlets.remove(&coord).unwrap_or_default()
                } else {
                    ChunkMeshletMask::default()
                };
                (
                    ChunkRemeshTaskKind::Geometry,
                    meshlets.union(lighting),
                )
            }
            ChunkRemeshTaskKind::Lighting => {
                if self.queue.remove(coord) {
                    let geometry =
                        self.geometry_meshlets.remove(&coord).unwrap_or_default();
                    (
                        ChunkRemeshTaskKind::Geometry,
                        meshlets.union(geometry),
                    )
                } else {
                    (ChunkRemeshTaskKind::Lighting, meshlets)
                }
            }
            ChunkRemeshTaskKind::Fluid => (kind, meshlets),
        }
    }

    pub(super) fn pop_renderable_background(
        &mut self,
        render_pool: &ChunkRenderPool,
        center: Option<IVec3>,
        allow_terrain: bool,
        allow_fluid: bool,
    ) -> Option<(IVec3, ChunkRemeshTaskKind, ChunkMeshletMask)> {
        const KIND_COUNT: usize = 3;

        for offset in 0..KIND_COUNT {
            let kind_index = (self.next_background_kind + offset) % KIND_COUNT;
            let next = match kind_index {
                0 if allow_fluid => self
                    .pop_renderable_fluid(render_pool, center)
                    .map(|(coord, meshlets)| (coord, ChunkRemeshTaskKind::Fluid, meshlets)),
                1 if allow_terrain => self
                    .pop_renderable_lighting(render_pool, center)
                    .map(|(coord, meshlets)| (coord, ChunkRemeshTaskKind::Lighting, meshlets)),
                2 if allow_terrain => self
                    .pop_renderable_geometry(render_pool, center)
                    .map(|(coord, meshlets)| (coord, ChunkRemeshTaskKind::Geometry, meshlets)),
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
        let coord = self.queue.pop()?;
        self.geometry_meshlets.remove(&coord);
        Some(coord)
    }

    #[cfg(test)]
    fn pop_fluid(&mut self) -> Option<IVec3> {
        let coord = self.fluid.pop()?;
        self.fluid_meshlets.remove(&coord);
        Some(coord)
    }

    #[cfg(test)]
    fn pop_lighting(&mut self) -> Option<IVec3> {
        let coord = self.lighting.pop()?;
        self.lighting_meshlets.remove(&coord);
        Some(coord)
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
    fn lighting_refresh_stays_separate_from_geometry_and_fluid() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_priority(coord);
        queue.enqueue_fluid_priority(coord);
        queue.enqueue_lighting_priority(coord);

        assert_eq!(queue.pop(), Some(coord));
        assert_eq!(queue.pop_lighting(), Some(coord));
        assert_eq!(queue.pop_fluid(), Some(coord));
    }

    #[test]
    fn chunk_lighting_change_refreshes_only_boundary_fluid_neighbors() {
        let coord = IVec3::new(2, 1, 3);
        let neighbor = coord + IVec3::X;
        let mut world = VoxelWorld::default();
        let mut fluid_chunk = VoxelChunk::empty();
        fluid_chunk.set_fluid(15, 8, 8, Some(FluidCell::source(0, 8)));
        world.insert_chunk(coord, fluid_chunk);
        world.insert_chunk(neighbor, VoxelChunk::empty());
        let mut queue = ChunkRemeshQueue::default();

        queue.enqueue_lighting_change(neighbor, &world);

        assert_eq!(queue.pop_fluid(), Some(coord));
        assert_eq!(queue.pop_fluid(), None);
        assert_eq!(queue.pop_lighting(), None);
    }

    #[test]
    fn voxel_edit_invalidates_only_chunks_whose_halo_contains_the_edit() {
        let mut queue = ChunkRemeshQueue::default();
        let interior = IVec3::new(4, 20, 6);
        queue.enqueue_voxel_edit(interior);

        assert_eq!(queue.pop_lighting(), None);
        assert_eq!(queue.pop(), Some(IVec3::new(0, 1, 0)));
        assert_eq!(queue.pop(), None);

        let corner = IVec3::new(15, 31, 15);
        queue.enqueue_voxel_edit(corner);
        let mut queued = Vec::new();
        while let Some(value) = queue.pop() {
            queued.push(value);
        }
        assert_eq!(queued.len(), 8);
    }

    #[test]
    fn voxel_lighting_change_queues_only_loaded_affected_terrain() {
        let mut queue = ChunkRemeshQueue::default();
        let mut world = VoxelWorld::default();
        let coord = IVec3::new(4, 2, -3);
        let origin = crate::voxel::coordinates::chunk_origin(coord);
        let position = origin + IVec3::new(4, 4, 4);
        let mut chunk = VoxelChunk::empty();
        chunk.set_block(
            4,
            4,
            4,
            Some(crate::voxel::cell::VoxelCell::new(
                "asteria:test",
                Default::default(),
            )),
        );
        world.insert_chunk(coord, chunk);

        queue.enqueue_lighting_voxel_change(position, &world);

        assert_eq!(queue.pop_lighting(), Some(coord));
        assert_eq!(queue.pop_lighting(), None);
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
