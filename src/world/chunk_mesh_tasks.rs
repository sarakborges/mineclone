use std::sync::Arc;

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::voxel::{
    coordinates::ChunkCoord,
    mesh_snapshot::{ChunkMeshDependencies, ChunkMeshSnapshot, ChunkSnapshotSource},
    meshlet::ChunkMeshletMask,
};

pub(crate) use super::presentation_snapshot::PresentationContentSnapshot as MeshContentSnapshot;
use super::{
    chunk_async_work::{ChunkAsyncWorkLimiter, ChunkAsyncWorkPermit},
    chunk_rendering::{BuiltChunkMesh, build_chunk_render_meshes},
    chunk_system_params::ChunkContent,
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
    revision::TaskInputRevision,
};

pub(crate) const MAX_MESH_TASKS_IN_FLIGHT: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChunkPresentationSource {
    coord: ChunkCoord,
    revisions: ChunkMeshDependencies,
}

impl ChunkPresentationSource {
    fn capture(coord: ChunkCoord, world: &ChunkMeshSnapshot) -> Self {
        Self {
            coord,
            revisions: world.dependencies(),
        }
    }

    pub(crate) fn is_current(&self, source: &impl ChunkSnapshotSource) -> bool {
        source.snapshot_chunk(self.coord).is_some() && self.revisions.is_current(source)
    }

    pub(crate) fn initial_catchup_meshlets_with(
        &self,
        source: &impl ChunkSnapshotSource,
        neighbor_is_visible: impl FnMut(IVec3) -> bool,
    ) -> ChunkMeshletMask {
        self.revisions
            .initial_catchup_meshlets_with(source, neighbor_is_visible)
    }
}

pub(crate) struct ChunkMeshTaskOutput {
    pub(crate) meshes: Vec<BuiltChunkMesh>,
    pub(crate) dependencies: ChunkPresentationSource,
}

#[derive(Resource, Default)]
pub(crate) struct PresentationScheduler {
    revision: TaskInputRevision,
    snapshot: Option<Arc<MeshContentSnapshot>>,
    pending: ChunkTaskQueue<ChunkMeshTaskOutput>,
}

/// Transitional compatibility name while initial-mesh call sites are migrated
/// to the explicit presentation scheduler owner.
pub(crate) type ChunkMeshTasks = PresentationScheduler;

impl PresentationScheduler {
    pub(crate) fn sync_snapshot(&mut self, content: &ChunkContent<'_>) {
        if self.snapshot.is_some() && !content.mesh_inputs_changed() {
            return;
        }

        self.revision = self.revision.next();
        self.snapshot = Some(Arc::new(MeshContentSnapshot::capture(content)));
    }

    pub(crate) fn revision(&self) -> TaskInputRevision {
        self.revision
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn contains(&self, coord: IVec3) -> bool {
        self.pending.contains(ChunkCoord::from_ivec3(coord))
    }

    pub(crate) fn schedule(
        &mut self,
        coord: IVec3,
        world: ChunkMeshSnapshot,
        limiter: &ChunkAsyncWorkLimiter,
    ) -> bool {
        self.schedule_with_permit(
            ChunkCoord::from_ivec3(coord),
            world,
            MAX_MESH_TASKS_IN_FLIGHT,
            || limiter.try_acquire_initial_mesh(),
        )
    }

    pub(crate) fn schedule_loading(
        &mut self,
        coord: IVec3,
        world: ChunkMeshSnapshot,
        limiter: &ChunkAsyncWorkLimiter,
    ) -> bool {
        self.schedule_with_permit(
            ChunkCoord::from_ivec3(coord),
            world,
            limiter.loading_queue_limit(),
            || limiter.try_acquire_loading_initial_mesh(),
        )
    }

    fn schedule_with_permit(
        &mut self,
        coord: ChunkCoord,
        world: ChunkMeshSnapshot,
        pending_limit: usize,
        acquire_permit: impl FnOnce() -> Option<ChunkAsyncWorkPermit>,
    ) -> bool {
        if self.pending.len() >= pending_limit || self.pending.contains(coord) {
            return false;
        }
        let Some(permit) = acquire_permit() else {
            return false;
        };

        let snapshot = self
            .snapshot
            .as_ref()
            .unwrap_or_else(|| panic!("chunk mesh snapshot must be prepared before scheduling"))
            .clone();
        let revision = self.revision;
        let dependencies = ChunkPresentationSource::capture(coord, &world);
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let _permit = permit;
            // The meshers now read central voxels directly and build a compact
            // lighting cache once when worthwhile. Flattening the captured halo
            // into another 18³ shell here only duplicates the same traversal.
            let context = snapshot.context(&world);
            ChunkMeshTaskOutput {
                meshes: build_chunk_render_meshes(coord.as_ivec3(), world.chunk(), &context),
                dependencies,
            }
        });

        self.pending.insert(coord, revision, task)
    }

    pub(crate) fn cancel_where(
        &mut self,
        mut predicate: impl FnMut(IVec3) -> bool,
    ) -> Vec<IVec3> {
        self.pending
            .cancel_where(|coord| predicate(coord.as_ivec3()))
            .into_iter()
            .map(ChunkCoord::as_ivec3)
            .collect()
    }

    pub(crate) fn cancel_farthest_where(
        &mut self,
        center: IVec3,
        mut predicate: impl FnMut(IVec3) -> bool,
    ) -> Option<IVec3> {
        self.pending
            .cancel_farthest_where(ChunkCoord::from_ivec3(center), |coord| {
                predicate(coord.as_ivec3())
            })
            .map(ChunkCoord::as_ivec3)
    }

    pub(crate) fn poll_ready_by_key<K: Ord>(
        &mut self,
        mut key: impl FnMut(IVec3) -> K,
    ) -> Option<CompletedChunkTask<ChunkMeshTaskOutput>> {
        self.pending
            .poll_ready_by_key(|coord| key(coord.as_ivec3()))
            .map(CompletedChunkTask::into_runtime)
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<ChunkMeshTaskOutput>> {
        self.pending.poll_ready().map(CompletedChunkTask::into_runtime)
    }
}
