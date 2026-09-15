use std::sync::Arc;

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::voxel::{
    fluid_mesh::ChunkFluidMesh,
    mesh_snapshot::{ChunkMeshDependencies, ChunkMeshSnapshot},
};

use super::{
    chunk_mesh_tasks::MeshContentSnapshot,
    chunk_rendering::{BuiltChunkMesh, build_chunk_fluid_remeshes, build_chunk_terrain_remeshes},
    chunk_system_params::ChunkContent,
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
};

pub(crate) const MAX_REMESH_TASKS_IN_FLIGHT: usize = 4;

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

pub(crate) struct ChunkRemeshTaskOutput {
    pub(crate) kind: ChunkRemeshTaskKind,
    pub(crate) meshes: ChunkRemeshTaskMeshes,
    pub(crate) dependencies: ChunkMeshDependencies,
}

#[derive(Resource, Default)]
pub(crate) struct ChunkRemeshTasks {
    revision: u64,
    snapshot: Option<Arc<MeshContentSnapshot>>,
    pending: ChunkTaskQueue<ChunkRemeshTaskOutput>,
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
        self.pending.len()
    }

    pub(crate) fn contains(&self, coord: IVec3) -> bool {
        self.pending.contains(coord)
    }

    pub(crate) fn schedule(
        &mut self,
        coord: IVec3,
        kind: ChunkRemeshTaskKind,
        world: ChunkMeshSnapshot,
    ) -> bool {
        if self.pending.len() >= MAX_REMESH_TASKS_IN_FLIGHT || self.pending.contains(coord) {
            return false;
        }

        let snapshot = self
            .snapshot
            .as_ref()
            .unwrap_or_else(|| panic!("chunk remesh snapshot must be prepared before scheduling"))
            .clone();
        let revision = self.revision;
        let dependencies = world.dependencies();
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

        self.pending.insert(coord, revision, task)
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<ChunkRemeshTaskOutput>> {
        self.pending.poll_ready()
    }
}
