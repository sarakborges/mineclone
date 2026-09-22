use std::sync::Arc;

use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    coordinates::chunk_origin,
    fluid::FluidCell,
    light::VoxelLight,
    meshlet::ChunkMeshletMask,
    read::VoxelRead,
    world::VoxelWorld,
};

type NeighborChunks = [[[Option<VoxelChunk>; 3]; 3]; 3];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChunkMeshDependencies {
    center: IVec3,
    content_revisions: [[[Option<u64>; 3]; 3]; 3],
    required_offsets: [[[bool; 3]; 3]; 3],
}

impl ChunkMeshDependencies {
    pub(crate) fn for_meshlets(mut self, meshlets: ChunkMeshletMask) -> Self {
        for offset_y in -1..=1 {
            for offset_z in -1..=1 {
                for offset_x in -1..=1 {
                    let offset = IVec3::new(offset_x, offset_y, offset_z);
                    self.required_offsets[(offset_y + 1) as usize]
                        [(offset_z + 1) as usize][(offset_x + 1) as usize] =
                        meshlets.depends_on_neighbor_offset(offset);
                }
            }
        }
        self
    }

    pub(crate) fn is_current(&self, world: &VoxelWorld) -> bool {
        for offset_y in -1..=1 {
            for offset_z in -1..=1 {
                for offset_x in -1..=1 {
                    let y = (offset_y + 1) as usize;
                    let z = (offset_z + 1) as usize;
                    let x = (offset_x + 1) as usize;
                    if !self.required_offsets[y][z][x] {
                        continue;
                    }
                    let expected = self.content_revisions[y][z][x];
                    let Some(expected) = expected else {
                        // Do not discard first-visible mesh when a previously absent neighbor
                        // loads during the async build. A cheap post-publication catch-up handles
                        // that new halo without starving the streaming frontier.
                        continue;
                    };
                    let coord = self.center + IVec3::new(offset_x, offset_y, offset_z);
                    if world.chunk_content_revision(coord) != Some(expected) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Initial meshes intentionally tolerate a missing neighbor becoming available.
    /// Once that first mesh is visible, reconcile its formerly absent halo rather
    /// than invalidating and repeatedly rescheduling the initial async task.
    #[cfg(test)]
    pub(crate) fn needs_initial_catchup(&self, world: &VoxelWorld) -> bool {
        !self
            .initial_catchup_meshlets_with(world, |_| true)
            .is_empty()
    }

    pub(crate) fn needs_initial_catchup_with(
        &self,
        world: &VoxelWorld,
        neighbor_is_visible: impl FnMut(IVec3) -> bool,
    ) -> bool {
        !self
            .initial_catchup_meshlets_with(world, neighbor_is_visible)
            .is_empty()
    }

    pub(crate) fn initial_catchup_meshlets_with(
        &self,
        world: &VoxelWorld,
        mut neighbor_is_visible: impl FnMut(IVec3) -> bool,
    ) -> ChunkMeshletMask {
        let mut meshlets = ChunkMeshletMask::default();

        for offset_y in -1..=1 {
            for offset_z in -1..=1 {
                for offset_x in -1..=1 {
                    if offset_x == 0 && offset_y == 0 && offset_z == 0 {
                        continue;
                    }
                    let y = (offset_y + 1) as usize;
                    let z = (offset_z + 1) as usize;
                    let x = (offset_x + 1) as usize;
                    if !self.required_offsets[y][z][x]
                        || self.content_revisions[y][z][x].is_some()
                    {
                        continue;
                    }

                    let offset = IVec3::new(offset_x, offset_y, offset_z);
                    let coord = self.center + offset;
                    if neighbor_is_visible(coord) && world.chunk(coord).is_some() {
                        meshlets = meshlets.union(
                            ChunkMeshletMask::for_dependency_offset(offset),
                        );
                    }
                }
            }
        }

        meshlets
    }
}

#[derive(Clone)]
pub(crate) struct ChunkMeshSnapshot {
    chunk_origin: IVec3,
    chunk: VoxelChunk,
    neighbor_chunks: Arc<NeighborChunks>,
    dependencies: ChunkMeshDependencies,
}

impl ChunkMeshSnapshot {
    pub(crate) fn capture(world: &VoxelWorld, coord: IVec3) -> Option<Self> {
        Self::capture_with_neighbor_filter_and_meshlets(
            world,
            coord,
            |_| true,
            ChunkMeshletMask::ALL,
        )
    }

    pub(crate) fn capture_with_neighbor_filter(
        world: &VoxelWorld,
        coord: IVec3,
        include_neighbor: impl FnMut(IVec3) -> bool,
    ) -> Option<Self> {
        Self::capture_with_neighbor_filter_and_meshlets(
            world,
            coord,
            include_neighbor,
            ChunkMeshletMask::ALL,
        )
    }

    pub(crate) fn capture_with_neighbor_filter_and_meshlets(
        world: &VoxelWorld,
        coord: IVec3,
        mut include_neighbor: impl FnMut(IVec3) -> bool,
        meshlets: ChunkMeshletMask,
    ) -> Option<Self> {
        let center_chunk = world.chunk(coord)?;
        let center_revision = world
            .chunk_content_revision(coord)
            .expect("loaded center chunk should have a content revision");
        let chunk = center_chunk.clone();
        let chunk_origin = chunk_origin(coord);
        let mut neighbor_chunks: NeighborChunks = std::array::from_fn(|_| {
            std::array::from_fn(|_| std::array::from_fn(|_| None))
        });
        let mut content_revisions = [[[None; 3]; 3]; 3];
        let required_offsets = [[[true; 3]; 3]; 3];
        content_revisions[1][1][1] = Some(center_revision);

        for offset_y in -1..=1 {
            for offset_z in -1..=1 {
                for offset_x in -1..=1 {
                    if offset_x == 0 && offset_y == 0 && offset_z == 0 {
                        continue;
                    }

                    let offset = IVec3::new(offset_x, offset_y, offset_z);
                    if !meshlets.depends_on_neighbor_offset(offset) {
                        continue;
                    }

                    let neighbor_coord = coord + offset;
                    if !include_neighbor(neighbor_coord) {
                        continue;
                    }
                    let Some(neighbor_chunk) = world.chunk(neighbor_coord) else {
                        continue;
                    };
                    let revision = world
                        .chunk_content_revision(neighbor_coord)
                        .expect("loaded neighbor chunk should have a content revision");
                    let y = (offset_y + 1) as usize;
                    let z = (offset_z + 1) as usize;
                    let x = (offset_x + 1) as usize;
                    neighbor_chunks[y][z][x] = Some(neighbor_chunk.clone());
                    content_revisions[y][z][x] = Some(revision);
                }
            }
        }

        Some(Self {
            chunk_origin,
            chunk,
            neighbor_chunks: Arc::new(neighbor_chunks),
            dependencies: ChunkMeshDependencies {
                center: coord,
                content_revisions,
                required_offsets,
            }
            .for_meshlets(meshlets),
        })
    }

    pub(crate) fn chunk(&self) -> &VoxelChunk {
        &self.chunk
    }

    pub(crate) fn dependencies(&self) -> ChunkMeshDependencies {
        self.dependencies
    }

    fn central_local(&self, position: IVec3) -> Option<IVec3> {
        let local = position - self.chunk_origin;
        inside_chunk(local).then_some(local)
    }

    fn neighbor_sample(
        &self,
        position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        let local = position - self.chunk_origin;
        let chunk_size = CHUNK_SIZE as i32;
        let offset = IVec3::new(
            local.x.div_euclid(chunk_size),
            local.y.div_euclid(chunk_size),
            local.z.div_euclid(chunk_size),
        );
        if offset.x.abs() > 1 || offset.y.abs() > 1 || offset.z.abs() > 1 {
            return None;
        }
        if offset == IVec3::ZERO {
            return self.chunk.sample_local(local.x, local.y, local.z);
        }

        let neighbor = self.neighbor_chunks[(offset.y + 1) as usize]
            [(offset.z + 1) as usize][(offset.x + 1) as usize]
            .as_ref()?;
        neighbor.sample_local(
            local.x.rem_euclid(chunk_size),
            local.y.rem_euclid(chunk_size),
            local.z.rem_euclid(chunk_size),
        )
    }

}

impl VoxelRead for ChunkMeshSnapshot {
    fn sample_at(
        &self,
        world_position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        if let Some(local) = self.central_local(world_position) {
            return self.chunk.sample_local(local.x, local.y, local.z);
        }

        self.neighbor_sample(world_position)
    }
}

fn inside_chunk(local: IVec3) -> bool {
    local.x >= 0
        && local.y >= 0
        && local.z >= 0
        && local.x < CHUNK_SIZE as i32
        && local.y < CHUNK_SIZE as i32
        && local.z < CHUNK_SIZE as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{
        cell::VoxelCell, texture_rotation::TextureRotation, world::VoxelWorld,
    };

    #[test]
    fn snapshot_preserves_central_chunk_and_one_voxel_halo() {
        let mut world = VoxelWorld::default();
        let coord = IVec3::ZERO;
        let mut center = VoxelChunk::empty();
        center.set_block(
            0,
            0,
            0,
            Some(VoxelCell::new("asteria:test", TextureRotation::default())),
        );
        world.insert_chunk(coord, center);

        let mut neighbor = VoxelChunk::empty();
        neighbor.set_block(
            0,
            0,
            0,
            Some(VoxelCell::new(
                "asteria:neighbor",
                TextureRotation::default(),
            )),
        );
        world.insert_chunk(IVec3::X, neighbor);

        let snapshot = ChunkMeshSnapshot::capture(&world, coord).expect("chunk should exist");
        assert_eq!(snapshot.block_id_at(IVec3::ZERO), Some("asteria:test"));
        assert_eq!(
            snapshot.block_id_at(IVec3::new(CHUNK_SIZE as i32, 0, 0)),
            Some("asteria:neighbor")
        );
        assert!(!snapshot.is_loaded_at(IVec3::new(CHUNK_SIZE as i32 + 1, 0, 0)));
        assert!(snapshot.dependencies().is_current(&world));
        assert!(!snapshot.dependencies().needs_initial_catchup(&world));
    }

    #[test]
    fn snapshot_reads_diagonal_neighbor_shell() {
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());

        let mut diagonal = VoxelChunk::empty();
        diagonal.set_block(
            0,
            0,
            0,
            Some(VoxelCell::new(
                "asteria:diagonal",
                TextureRotation::default(),
            )),
        );
        world.insert_chunk(IVec3::new(1, 1, 1), diagonal);

        let snapshot =
            ChunkMeshSnapshot::capture(&world, IVec3::ZERO).expect("chunk should exist");
        let edge = CHUNK_SIZE as i32;
        assert_eq!(
            snapshot.block_id_at(IVec3::new(edge, edge, edge)),
            Some("asteria:diagonal")
        );
    }

    #[test]
    fn snapshot_filter_hides_resident_neighbor_until_it_is_visible() {
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());

        let mut neighbor = VoxelChunk::empty();
        neighbor.set_block(
            0,
            0,
            0,
            Some(VoxelCell::new(
                "asteria:hidden_neighbor",
                TextureRotation::default(),
            )),
        );
        world.insert_chunk(IVec3::X, neighbor);

        let snapshot = ChunkMeshSnapshot::capture_with_neighbor_filter(
            &world,
            IVec3::ZERO,
            |_| false,
        )
        .expect("center chunk should exist");
        let edge = CHUNK_SIZE as i32;

        assert_eq!(snapshot.block_id_at(IVec3::new(edge, 0, 0)), None);
        assert!(!snapshot.dependencies().needs_initial_catchup_with(&world, |_| false));
        assert!(snapshot.dependencies().needs_initial_catchup_with(&world, |coord| {
            coord == IVec3::X
        }));
    }

    #[test]
    fn snapshot_preserves_captured_neighbor_state() {
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());

        let edge = CHUNK_SIZE as i32;
        let mut neighbor = VoxelChunk::empty();
        neighbor.set_block(
            0,
            0,
            0,
            Some(VoxelCell::new("asteria:before", TextureRotation::default())),
        );
        world.insert_chunk(IVec3::X, neighbor);

        let snapshot =
            ChunkMeshSnapshot::capture(&world, IVec3::ZERO).expect("chunk should exist");
        world.set_block_at(
            IVec3::new(edge, 0, 0),
            Some(VoxelCell::new("asteria:after", TextureRotation::default())),
        );
        assert_eq!(
            snapshot.block_id_at(IVec3::new(edge, 0, 0)),
            Some("asteria:before")
        );
        assert!(!snapshot.dependencies().is_current(&world));
    }

    #[test]
    fn snapshot_dependencies_allow_new_neighbors_but_detect_content_changes() {
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());
        let snapshot =
            ChunkMeshSnapshot::capture(&world, IVec3::ZERO).expect("chunk should exist");
        assert!(snapshot.dependencies().is_current(&world));
        assert!(!snapshot.dependencies().needs_initial_catchup(&world));

        world.insert_chunk(IVec3::X, VoxelChunk::empty());
        assert!(snapshot.dependencies().is_current(&world));
        assert!(snapshot.dependencies().needs_initial_catchup(&world));

        let refreshed =
            ChunkMeshSnapshot::capture(&world, IVec3::ZERO).expect("chunk should exist");
        assert!(!refreshed.dependencies().needs_initial_catchup(&world));
        world.set_block_at(
            IVec3::new(CHUNK_SIZE as i32, 0, 0),
            Some(VoxelCell::new(
                "asteria:changed",
                TextureRotation::default(),
            )),
        );
        assert!(!refreshed.dependencies().is_current(&world));
    }

    #[test]
    fn snapshot_dependencies_ignore_lighting_revision_churn() {
        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());
        let snapshot =
            ChunkMeshSnapshot::capture(&world, IVec3::ZERO).expect("chunk should exist");
        let mesh_revision = world
            .chunk_mesh_revision(IVec3::ZERO)
            .expect("chunk should have a mesh revision");

        assert!(world.clear_chunk_light(IVec3::ZERO));
        assert!(
            world
                .chunk_mesh_revision(IVec3::ZERO)
                .expect("chunk should have a mesh revision")
                > mesh_revision
        );
        assert!(snapshot.dependencies().is_current(&world));
    }


}