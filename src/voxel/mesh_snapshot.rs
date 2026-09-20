use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    coordinates::chunk_origin,
    fluid::FluidCell,
    light::VoxelLight,
    read::VoxelRead,
    world::VoxelWorld,
};

const HALO: i32 = 1;
const SNAPSHOT_SIDE: usize = CHUNK_SIZE + 2;
const SNAPSHOT_FACE_AREA: usize = SNAPSHOT_SIDE * SNAPSHOT_SIDE;
const INTERIOR_SHELL_LAYER: usize = SNAPSHOT_SIDE * 2 + CHUNK_SIZE * 2;
const SHELL_VOLUME: usize = SNAPSHOT_FACE_AREA * 2 + INTERIOR_SHELL_LAYER * CHUNK_SIZE;

type NeighborChunks = [[[Option<VoxelChunk>; 3]; 3]; 3];

#[derive(Clone, Copy, Default)]
struct ShellSample {
    cell: Option<VoxelCell>,
    fluid: Option<FluidCell>,
    light: VoxelLight,
    loaded: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChunkMeshDependencies {
    center: IVec3,
    content_revisions: [[[Option<u64>; 3]; 3]; 3],
}

impl ChunkMeshDependencies {
    pub(crate) fn is_current(&self, world: &VoxelWorld) -> bool {
        for offset_y in -1..=1 {
            for offset_z in -1..=1 {
                for offset_x in -1..=1 {
                    let expected = self.content_revisions[(offset_y + 1) as usize]
                        [(offset_z + 1) as usize][(offset_x + 1) as usize];
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
        self.needs_initial_catchup_with(world, |_| true)
    }

    pub(crate) fn needs_initial_catchup_with(
        &self,
        world: &VoxelWorld,
        mut neighbor_is_visible: impl FnMut(IVec3) -> bool,
    ) -> bool {
        for offset_y in -1..=1 {
            for offset_z in -1..=1 {
                for offset_x in -1..=1 {
                    if offset_x == 0 && offset_y == 0 && offset_z == 0 {
                        continue;
                    }
                    if self.content_revisions[(offset_y + 1) as usize]
                        [(offset_z + 1) as usize][(offset_x + 1) as usize]
                        .is_some()
                    {
                        continue;
                    }
                    let coord = self.center + IVec3::new(offset_x, offset_y, offset_z);
                    if neighbor_is_visible(coord) && world.chunk(coord).is_some() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

pub(crate) struct ChunkMeshSnapshot {
    chunk_origin: IVec3,
    chunk: VoxelChunk,
    neighbor_chunks: Option<Box<NeighborChunks>>,
    shell: Option<Box<[ShellSample]>>,
    dependencies: ChunkMeshDependencies,
}

impl ChunkMeshSnapshot {
    pub(crate) fn capture(world: &VoxelWorld, coord: IVec3) -> Option<Self> {
        Self::capture_with_neighbor_filter(world, coord, |_| true)
    }

    pub(crate) fn capture_with_neighbor_filter(
        world: &VoxelWorld,
        coord: IVec3,
        mut include_neighbor: impl FnMut(IVec3) -> bool,
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
        content_revisions[1][1][1] = Some(center_revision);

        for offset_y in -1..=1 {
            for offset_z in -1..=1 {
                for offset_x in -1..=1 {
                    if offset_x == 0 && offset_y == 0 && offset_z == 0 {
                        continue;
                    }

                    let neighbor_coord = coord + IVec3::new(offset_x, offset_y, offset_z);
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
            neighbor_chunks: Some(Box::new(neighbor_chunks)),
            shell: None,
            dependencies: ChunkMeshDependencies {
                center: coord,
                content_revisions,
            },
        })
    }

    pub(crate) fn materialize_shell(mut self) -> Self {
        if self.shell.is_some() {
            return self;
        }

        let neighbor_chunks = self
            .neighbor_chunks
            .take()
            .expect("unmaterialized chunk mesh snapshot must retain neighbor chunks");
        self.shell = Some(capture_shell(&neighbor_chunks));
        self
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

    fn snapshot_local(&self, position: IVec3) -> Option<IVec3> {
        let snapshot_origin = self.chunk_origin - IVec3::splat(HALO);
        let local = position - snapshot_origin;
        if local.x < 0
            || local.y < 0
            || local.z < 0
            || local.x >= SNAPSHOT_SIDE as i32
            || local.y >= SNAPSHOT_SIDE as i32
            || local.z >= SNAPSHOT_SIDE as i32
        {
            return None;
        }
        Some(local)
    }

    fn shell_index(&self, position: IVec3) -> Option<usize> {
        let local = self.snapshot_local(position)?;
        shell_index_from_snapshot_coords(local.x as usize, local.y as usize, local.z as usize)
    }

    fn neighbor_sample(
        &self,
        position: IVec3,
    ) -> Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)> {
        let local = self.snapshot_local(position)?;
        shell_index_from_snapshot_coords(local.x as usize, local.y as usize, local.z as usize)?;
        let neighbor_chunks = self.neighbor_chunks.as_ref()?;
        let (chunk_x, local_x) = shell_axis(local.x as usize);
        let (chunk_y, local_y) = shell_axis(local.y as usize);
        let (chunk_z, local_z) = shell_axis(local.z as usize);
        neighbor_chunks[chunk_y][chunk_z][chunk_x]
            .as_ref()?
            .sample_local(local_x, local_y, local_z)
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

        if let Some(shell) = &self.shell {
            let sample = shell[self.shell_index(world_position)?];
            return sample
                .loaded
                .then_some((sample.cell, sample.fluid, sample.light));
        }

        self.neighbor_sample(world_position)
    }
}

fn capture_shell(neighbor_chunks: &NeighborChunks) -> Box<[ShellSample]> {
    let last = SNAPSHOT_SIDE - 1;
    let capture_shell_voxel = |x: usize, y: usize, z: usize| {
        let (chunk_x, local_x) = shell_axis(x);
        let (chunk_y, local_y) = shell_axis(y);
        let (chunk_z, local_z) = shell_axis(z);
        let Some(neighbor_chunk) = neighbor_chunks[chunk_y][chunk_z][chunk_x].as_ref() else {
            return ShellSample::default();
        };
        let Some((cell, fluid, light)) = neighbor_chunk.sample_local(local_x, local_y, local_z)
        else {
            return ShellSample::default();
        };

        ShellSample {
            cell,
            fluid,
            light,
            loaded: true,
        }
    };
    let mut shell = Vec::with_capacity(SHELL_VOLUME);

    for z in 0..SNAPSHOT_SIDE {
        for x in 0..SNAPSHOT_SIDE {
            shell.push(capture_shell_voxel(x, 0, z));
        }
    }
    for z in 0..SNAPSHOT_SIDE {
        for x in 0..SNAPSHOT_SIDE {
            shell.push(capture_shell_voxel(x, last, z));
        }
    }
    for y in 1..last {
        for x in 0..SNAPSHOT_SIDE {
            shell.push(capture_shell_voxel(x, y, 0));
        }
        for x in 0..SNAPSHOT_SIDE {
            shell.push(capture_shell_voxel(x, y, last));
        }
        for z in 1..last {
            shell.push(capture_shell_voxel(0, y, z));
            shell.push(capture_shell_voxel(last, y, z));
        }
    }
    debug_assert_eq!(shell.len(), SHELL_VOLUME);
    shell.into_boxed_slice()
}

fn shell_axis(coordinate: usize) -> (usize, i32) {
    let last = SNAPSHOT_SIDE - 1;

    if coordinate == 0 {
        (0, CHUNK_SIZE as i32 - 1)
    } else if coordinate == last {
        (2, 0)
    } else {
        (1, coordinate as i32 - 1)
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

fn shell_index_from_snapshot_coords(x: usize, y: usize, z: usize) -> Option<usize> {
    let last = SNAPSHOT_SIDE - 1;

    if y == 0 {
        return Some(x + z * SNAPSHOT_SIDE);
    }
    if y == last {
        return Some(SNAPSHOT_FACE_AREA + x + z * SNAPSHOT_SIDE);
    }

    let layer = SNAPSHOT_FACE_AREA * 2 + (y - 1) * INTERIOR_SHELL_LAYER;
    if z == 0 {
        return Some(layer + x);
    }
    if z == last {
        return Some(layer + SNAPSHOT_SIDE + x);
    }
    if x == 0 {
        return Some(layer + SNAPSHOT_SIDE * 2 + (z - 1) * 2);
    }
    if x == last {
        return Some(layer + SNAPSHOT_SIDE * 2 + (z - 1) * 2 + 1);
    }

    None
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
    fn materialized_snapshot_preserves_captured_neighbor_state() {
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
        let snapshot = snapshot.materialize_shell();

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

    #[test]
    fn compact_shell_index_covers_each_shell_voxel_once() {
        let mut seen = vec![false; SHELL_VOLUME];
        let mut count = 0;

        for y in 0..SNAPSHOT_SIDE {
            for z in 0..SNAPSHOT_SIDE {
                for x in 0..SNAPSHOT_SIDE {
                    let Some(index) = shell_index_from_snapshot_coords(x, y, z) else {
                        continue;
                    };
                    assert!(index < SHELL_VOLUME);
                    assert!(!seen[index]);
                    seen[index] = true;
                    count += 1;
                }
            }
        }

        assert_eq!(count, SHELL_VOLUME);
        assert!(seen.into_iter().all(|value| value));
    }
}