use std::sync::Arc;

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::voxel::{
    coordinates::ChunkCoord,
    mesh_snapshot::ChunkMeshSnapshot,
    meshlet::ChunkMeshletMask,
};

pub(crate) use super::presentation_snapshot::PresentationContentSnapshot as MeshContentSnapshot;
use super::{
    chunk_async_work::{ChunkAsyncWorkLimiter, ChunkAsyncWorkPermit},
    chunk_rendering::{BuiltChunkMesh, build_chunk_render_meshes},
    chunk_system_params::ChunkContent,
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
    presentation_snapshot::{
        ChunkPresentationSource, PresentationLightingRevisions, PresentationLightingSource,
    },
    revision::TaskInputRevision,
};

pub(crate) const MAX_MESH_TASKS_IN_FLIGHT: usize = 8;

pub(crate) struct ChunkMeshTaskOutput {
    pub(crate) meshes: Vec<BuiltChunkMesh>,
    pub(crate) content_source: ChunkPresentationSource,
    pub(crate) lighting_source: PresentationLightingSource,
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
        lighting_revisions: &PresentationLightingRevisions,
        limiter: &ChunkAsyncWorkLimiter,
    ) -> bool {
        self.schedule_with_permit(
            ChunkCoord::from_ivec3(coord),
            world,
            lighting_revisions,
            MAX_MESH_TASKS_IN_FLIGHT,
            || limiter.try_acquire_initial_mesh(),
        )
    }

    pub(crate) fn schedule_loading(
        &mut self,
        coord: IVec3,
        world: ChunkMeshSnapshot,
        lighting_revisions: &PresentationLightingRevisions,
        limiter: &ChunkAsyncWorkLimiter,
    ) -> bool {
        self.schedule_with_permit(
            ChunkCoord::from_ivec3(coord),
            world,
            lighting_revisions,
            limiter.loading_queue_limit(),
            || limiter.try_acquire_loading_initial_mesh(),
        )
    }

    fn schedule_with_permit(
        &mut self,
        coord: ChunkCoord,
        world: ChunkMeshSnapshot,
        lighting_revisions: &PresentationLightingRevisions,
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
        let content_source = ChunkPresentationSource::capture(coord, &world);
        let lighting_source =
            lighting_revisions.capture(coord.as_ivec3(), ChunkMeshletMask::ALL);
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let _permit = permit;
            // The meshers now read central voxels directly and build a compact
            // lighting cache once when worthwhile. Flattening the captured halo
            // into another 18³ shell here only duplicates the same traversal.
            let context = snapshot.context(&world);
            ChunkMeshTaskOutput {
                meshes: build_chunk_render_meshes(coord.as_ivec3(), world.chunk(), &context),
                content_source,
                lighting_source,
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
