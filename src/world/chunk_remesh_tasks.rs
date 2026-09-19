use std::sync::{Arc, RwLock};

use bevy::{
    platform::collections::HashMap,
    prelude::*,
    tasks::AsyncComputeTaskPool,
};

use crate::voxel::{
    fluid_mesh::ChunkFluidMesh,
    mesh_snapshot::{ChunkMeshDependencies, ChunkMeshSnapshot},
    world::VoxelWorld,
};

use super::{
    chunk_mesh_tasks::MeshContentSnapshot,
    chunk_rendering::{BuiltChunkMesh, build_chunk_fluid_remeshes, build_chunk_terrain_remeshes},
    chunk_system_params::ChunkContent,
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
};

const MAX_TERRAIN_REMESH_TASKS_IN_FLIGHT: usize = 4;
const MAX_FLUID_REMESH_TASKS_IN_FLIGHT: usize = 4;

type SharedLightingRevisions = Arc<RwLock<HashMap<IVec3, u64>>>;

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
    expected: Vec<(IVec3, u64)>,
    revisions: SharedLightingRevisions,
}

impl LightingRemeshDependencies {
    fn capture(center: IVec3, revisions: &SharedLightingRevisions) -> Self {
        let current = revisions
            .read()
            .expect("lighting remesh revision tracker should not be poisoned");
        let mut expected = Vec::with_capacity(27);

        for y in -1..=1 {
            for z in -1..=1 {
                for x in -1..=1 {
                    let coord = center + IVec3::new(x, y, z);
                    expected.push((coord, current.get(&coord).copied().unwrap_or(0)));
                }
            }
        }

        drop(current);
        Self {
            expected,
            revisions: Arc::clone(revisions),
        }
    }

    fn is_current(&self) -> bool {
        let current = self
            .revisions
            .read()
            .expect("lighting remesh revision tracker should not be poisoned");
        self.expected
            .iter()
            .all(|(coord, expected)| current.get(coord).copied().unwrap_or(0) == *expected)
    }
}

pub(crate) struct ChunkRemeshDependencies {
    content: ChunkMeshDependencies,
    lighting: LightingRemeshDependencies,
}

impl ChunkRemeshDependencies {
    fn capture(
        center: IVec3,
        world: &ChunkMeshSnapshot,
        revisions: &SharedLightingRevisions,
    ) -> Self {
        Self {
            content: world.dependencies(),
            // Both terrain and fluid meshes bake light from the captured chunk
            // and its halo. Neither may overwrite a newer lighting result.
            lighting: LightingRemeshDependencies::capture(center, revisions),
        }
    }

    pub(crate) fn content_is_current(&self, world: &VoxelWorld) -> bool {
        self.content.is_current(world)
    }

    pub(crate) fn lighting_is_current(&self) -> bool {
        self.lighting.is_current()
    }

}

pub(crate) struct ChunkRemeshTaskOutput {
    pub(crate) kind: ChunkRemeshTaskKind,
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
    lighting_revisions: SharedLightingRevisions,
}

impl Default for ChunkRemeshTasks {
    fn default() -> Self {
        Self {
            revision: 0,
            snapshot: None,
            terrain_pending: ChunkTaskQueue::default(),
            fluid_pending: ChunkTaskQueue::default(),
            poll_fluid_first: true,
            lighting_revisions: Arc::new(RwLock::new(HashMap::default())),
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

    pub(crate) fn bump_lighting_revisions(
        &mut self,
        coords: impl IntoIterator<Item = IVec3>,
    ) {
        let mut revisions = self
            .lighting_revisions
            .write()
            .expect("lighting remesh revision tracker should not be poisoned");
        for coord in coords {
            let next = revisions
                .get(&coord)
                .copied()
                .unwrap_or(0)
                .wrapping_add(1)
                .max(1);
            revisions.insert(coord, next);
        }
    }

    pub(crate) fn remove_lighting_revision(&mut self, coord: IVec3) {
        self.lighting_revisions
            .write()
            .expect("lighting remesh revision tracker should not be poisoned")
            .remove(&coord);
    }

    pub(crate) fn schedule(
        &mut self,
        coord: IVec3,
        kind: ChunkRemeshTaskKind,
        world: ChunkMeshSnapshot,
    ) -> bool {
        if !self.can_schedule(kind) || self.contains(coord, kind) {
            return false;
        }

        let snapshot = self
            .snapshot
            .as_ref()
            .unwrap_or_else(|| panic!("chunk remesh snapshot must be prepared before scheduling"))
            .clone();
        let revision = self.revision;
        let dependencies = ChunkRemeshDependencies::capture(
            coord,
            &world,
            &self.lighting_revisions,
        );
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let world = world.materialize_shell();
            let context = snapshot.context(&world);
            let meshes = match kind {
                ChunkRemeshTaskKind::Geometry | ChunkRemeshTaskKind::Lighting => {
                    ChunkRemeshTaskMeshes::Geometry(build_chunk_terrain_remeshes(
                        coord,
                        world.chunk(),
                        &context,
                    ))
                }
                ChunkRemeshTaskKind::Fluid => ChunkRemeshTaskMeshes::Fluid(
                    build_chunk_fluid_remeshes(coord, world.chunk(), &context),
                ),
            };

            ChunkRemeshTaskOutput {
                kind,
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
        let dependencies =
            LightingRemeshDependencies::capture(center, &tasks.lighting_revisions);

        assert!(dependencies.is_current());
        tasks.bump_lighting_revisions([center + IVec3::X]);
        assert!(!dependencies.is_current());
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
            &snapshot,
            &tasks.lighting_revisions,
        );

        assert!(dependencies.content_is_current(&world));
        assert!(dependencies.lighting_is_current());

        tasks.bump_lighting_revisions([center + IVec3::X]);

        assert!(dependencies.content_is_current(&world));
        assert!(!dependencies.lighting_is_current());
    }
}
