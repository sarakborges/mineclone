use std::sync::Arc;

use bevy::{
    platform::collections::HashMap,
    prelude::*,
    tasks::AsyncComputeTaskPool,
};

use crate::voxel::{
    coordinates::visit_chunk_coords_whose_voxel_halo_contains,
    fluid_mesh::ChunkFluidMesh,
    mesh_snapshot::{ChunkMeshDependencies, ChunkMeshSnapshot},
    meshlet::ChunkMeshletMask,
    world::VoxelWorld,
};

use super::{
    chunk_async_work::ChunkAsyncWorkLimiter,
    chunk_mesh_tasks::MeshContentSnapshot,
    chunk_rendering::{
        BuiltChunkMesh, build_chunk_fluid_meshlet_remeshes,
        build_chunk_terrain_meshlet_remeshes,
    },
    chunk_system_params::ChunkContent,
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
};

const MAX_TERRAIN_REMESH_TASKS_IN_FLIGHT: usize = 4;
const MAX_FLUID_REMESH_TASKS_IN_FLIGHT: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChunkRemeshTaskKind {
    Geometry,
    Lighting,
    Fluid,
}

pub(crate) enum ChunkRemeshTaskMeshes {
    Geometry(Vec<BuiltChunkMesh>),
    Fluid(Vec<ChunkFluidMesh>),
}

#[derive(Clone)]
struct LightingRemeshDependencies {
    center: IVec3,
    meshlets: ChunkMeshletMask,
    expected: [u64; 8],
}

impl LightingRemeshDependencies {
    fn capture(
        center: IVec3,
        meshlets: ChunkMeshletMask,
        revisions: &HashMap<IVec3, [u64; 8]>,
    ) -> Self {
        Self {
            center,
            meshlets,
            expected: revisions.get(&center).copied().unwrap_or([0; 8]),
        }
    }

    fn is_current(&self, current: &HashMap<IVec3, [u64; 8]>) -> bool {
        let revisions = current.get(&self.center).copied().unwrap_or([0; 8]);
        (0..8).all(|index| {
            !self.meshlets.contains_index(index) || revisions[index] == self.expected[index]
        })
    }
}

pub(crate) struct ChunkRemeshDependencies {
    content: ChunkMeshDependencies,
    lighting: LightingRemeshDependencies,
}

impl ChunkRemeshDependencies {
    fn capture(
        center: IVec3,
        meshlets: ChunkMeshletMask,
        world: &ChunkMeshSnapshot,
        revisions: &HashMap<IVec3, [u64; 8]>,
    ) -> Self {
        Self {
            content: world.dependencies().for_meshlets(meshlets),
            // Revision slots are already expanded by the one-voxel lighting
            // halo, so a partial task only depends on the meshlets it rebuilds.
            lighting: LightingRemeshDependencies::capture(center, meshlets, revisions),
        }
    }

    pub(crate) fn content_is_current(&self, world: &VoxelWorld) -> bool {
        self.content.is_current(world)
    }

    pub(crate) fn lighting_is_current(&self, tasks: &ChunkRemeshTasks) -> bool {
        self.lighting.is_current(&tasks.lighting_revisions)
    }

}

pub(crate) struct ChunkRemeshTaskOutput {
    pub(crate) kind: ChunkRemeshTaskKind,
    pub(crate) meshlets: ChunkMeshletMask,
    pub(crate) meshes: ChunkRemeshTaskMeshes,
    pub(crate) dependencies: ChunkRemeshDependencies,
}

#[derive(Resource)]
pub(crate) struct ChunkRemeshTasks {
    revision: u64,
    snapshot: Option<Arc<MeshContentSnapshot>>,
    terrain_pending: ChunkTaskQueue<ChunkRemeshTaskOutput>,
    fluid_pending: ChunkTaskQueue<ChunkRemeshTaskOutput>,
    poll_fluid_first: bool,
    lighting_revisions: HashMap<IVec3, [u64; 8]>,
}

impl Default for ChunkRemeshTasks {
    fn default() -> Self {
        Self {
            revision: 0,
            snapshot: None,
            terrain_pending: ChunkTaskQueue::default(),
            fluid_pending: ChunkTaskQueue::default(),
            poll_fluid_first: true,
            lighting_revisions: HashMap::default(),
        }
    }
}

impl ChunkRemeshTasks {
    pub(crate) fn sync_snapshot(&mut self, content: &ChunkContent<'_>) {
        if self.snapshot.is_some() && !content.mesh_inputs_changed() {
            return;
        }

        self.revision = self.revision.wrapping_add(1).max(1);
        self.snapshot = Some(Arc::new(MeshContentSnapshot::from_content(content)));
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.terrain_pending.len() + self.fluid_pending.len()
    }

    pub(crate) fn can_schedule(&self, kind: ChunkRemeshTaskKind) -> bool {
        match kind {
            ChunkRemeshTaskKind::Geometry | ChunkRemeshTaskKind::Lighting => {
                self.terrain_pending.len() < MAX_TERRAIN_REMESH_TASKS_IN_FLIGHT
            }
            ChunkRemeshTaskKind::Fluid => {
                self.fluid_pending.len() < MAX_FLUID_REMESH_TASKS_IN_FLIGHT
            }
        }
    }

    pub(crate) fn contains(&self, coord: IVec3, kind: ChunkRemeshTaskKind) -> bool {
        match kind {
            ChunkRemeshTaskKind::Geometry | ChunkRemeshTaskKind::Lighting => {
                self.terrain_pending.contains(coord)
            }
            ChunkRemeshTaskKind::Fluid => self.fluid_pending.contains(coord),
        }
    }

    pub(crate) fn bump_lighting_revisions_for_positions(
        &mut self,
        world: &VoxelWorld,
        positions: impl IntoIterator<Item = IVec3>,
    ) {
        let revisions = &mut self.lighting_revisions;
        for position in positions {
            visit_chunk_coords_whose_voxel_halo_contains(position, |coord| {
                if world.chunk(coord).is_none() {
                    return;
                }

                let meshlets = ChunkMeshletMask::for_world_position(coord, position);
                let entry = revisions.entry(coord).or_insert([0; 8]);
                for (index, revision) in entry.iter_mut().enumerate() {
                    if !meshlets.contains_index(index) {
                        continue;
                    }
                    *revision = revision.wrapping_add(1).max(1);
                }
            });
        }
    }

    pub(crate) fn remove_lighting_revision(&mut self, coord: IVec3) {
        self.lighting_revisions.remove(&coord);
    }

    pub(crate) fn schedule(
        &mut self,
        coord: IVec3,
        kind: ChunkRemeshTaskKind,
        meshlets: ChunkMeshletMask,
        world: ChunkMeshSnapshot,
        limiter: &ChunkAsyncWorkLimiter,
    ) -> bool {
        if !self.can_schedule(kind) || self.contains(coord, kind) {
            return false;
        }
        let Some(permit) = limiter.try_acquire() else {
            return false;
        };

        let snapshot = self
            .snapshot
            .as_ref()
            .unwrap_or_else(|| panic!("chunk remesh snapshot must be prepared before scheduling"))
            .clone();
        let revision = self.revision;
        let dependencies = ChunkRemeshDependencies::capture(
            coord,
            meshlets,
            &world,
            &self.lighting_revisions,
        );
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let _permit = permit;
            let world = world.materialize_shell();
            let context = snapshot.context(&world);
            let meshes = match kind {
                ChunkRemeshTaskKind::Geometry | ChunkRemeshTaskKind::Lighting => {
                    ChunkRemeshTaskMeshes::Geometry(build_chunk_terrain_meshlet_remeshes(
                        coord,
                        world.chunk(),
                        &context,
                        meshlets,
                    ))
                }
                ChunkRemeshTaskKind::Fluid => ChunkRemeshTaskMeshes::Fluid(
                    build_chunk_fluid_meshlet_remeshes(
                        coord,
                        world.chunk(),
                        &context,
                        meshlets,
                    ),
                ),
            };

            ChunkRemeshTaskOutput {
                kind,
                meshlets,
                meshes,
                dependencies,
            }
        });

        match kind {
            ChunkRemeshTaskKind::Geometry | ChunkRemeshTaskKind::Lighting => {
                self.terrain_pending.insert(coord, revision, task)
            }
            ChunkRemeshTaskKind::Fluid => self.fluid_pending.insert(coord, revision, task),
        }
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<ChunkRemeshTaskOutput>> {
        let fluid_first = self.poll_fluid_first;
        let ready = if fluid_first {
            self.fluid_pending
                .poll_ready()
                .or_else(|| self.terrain_pending.poll_ready())
        } else {
            self.terrain_pending
                .poll_ready()
                .or_else(|| self.fluid_pending.poll_ready())
        };
        if ready.is_some() {
            self.poll_fluid_first = !fluid_first;
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::chunk::VoxelChunk;

    #[test]
    fn fluid_and_terrain_remeshes_can_share_a_chunk_in_flight() {
        let tasks = ChunkRemeshTasks::default();
        let coord = IVec3::new(3, 2, 5);

        assert!(!tasks.contains(coord, ChunkRemeshTaskKind::Fluid));
        assert!(!tasks.contains(coord, ChunkRemeshTaskKind::Geometry));
        assert!(tasks.can_schedule(ChunkRemeshTaskKind::Fluid));
        assert!(tasks.can_schedule(ChunkRemeshTaskKind::Geometry));
    }

    #[test]
    fn lighting_dependencies_detect_halo_revision_changes() {
        let mut tasks = ChunkRemeshTasks::default();
        let center = IVec3::new(3, 2, 5);
        let dependencies = LightingRemeshDependencies::capture(
            center,
            ChunkMeshletMask::ALL,
            &tasks.lighting_revisions,
        );

        assert!(dependencies.is_current(&tasks.lighting_revisions));
        let mut world = VoxelWorld::default();
        world.insert_chunk(center, VoxelChunk::empty());
        tasks.bump_lighting_revisions_for_positions(&world, [center * 16 + IVec3::new(4, 4, 4)]);
        assert!(!dependencies.is_current(&tasks.lighting_revisions));
    }

    #[test]
    fn remesh_dependencies_track_content_and_lighting_freshness_independently() {
        let center = IVec3::new(3, 2, 5);
        let mut world = VoxelWorld::default();
        world.insert_chunk(center, VoxelChunk::empty());
        let snapshot = ChunkMeshSnapshot::capture(&world, center).unwrap();
        let mut tasks = ChunkRemeshTasks::default();
        let dependencies = ChunkRemeshDependencies::capture(
            center,
            ChunkMeshletMask::ALL,
            &snapshot,
            &tasks.lighting_revisions,
        );

        assert!(dependencies.content_is_current(&world));
        assert!(dependencies.lighting_is_current(&tasks));

        tasks.bump_lighting_revisions_for_positions(
            &world,
            [center * 16 + IVec3::new(4, 4, 4)],
        );

        assert!(dependencies.content_is_current(&world));
        assert!(!dependencies.lighting_is_current(&tasks));
    }
}
