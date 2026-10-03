use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::voxel::{chunk::VoxelChunk, coordinates::ChunkCoord};

use super::{
    chunk_async_work::{ChunkAsyncWorkLimiter, ChunkAsyncWorkPermit},
    chunk_system_params::{ChunkContent, ChunkGeneration},
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
    generation_job::ChunkGenerationJob,
    generation_snapshot::GenerationSnapshot,
    revision::TaskInputRevision,
};

pub(crate) const MAX_GENERATION_TASKS_IN_FLIGHT: usize = 8;
const SLOW_GENERATION_SCHEDULER_WARNING: Duration = Duration::from_millis(8);

#[derive(Resource, Default)]
pub(crate) struct GenerationScheduler {
    revision: TaskInputRevision,
    snapshot: Option<Arc<GenerationSnapshot>>,
    pending: ChunkTaskQueue<VoxelChunk>,
}

impl GenerationScheduler {
    pub(crate) fn sync_snapshot(
        &mut self,
        generation: &ChunkGeneration<'_>,
        content: &ChunkContent<'_>,
    ) {
        let inputs_changed = generation.inputs_changed() || content.generation_inputs_changed();
        if self.snapshot.is_some() && !inputs_changed {
            return;
        }

        let started = Instant::now();
        let fresh_feature_caches =
            self.snapshot.is_some() && generation.world_generation.is_changed();
        self.revision = self.revision.next();
        self.snapshot = Some(Arc::new(GenerationSnapshot::capture(
            generation,
            content,
            fresh_feature_caches,
        )));
        let elapsed = started.elapsed();
        if elapsed >= SLOW_GENERATION_SCHEDULER_WARNING {
            warn!(
                "slow streaming generation snapshot refresh: elapsed_us={} fresh_feature_caches={} revision={:?}",
                elapsed.as_micros(),
                fresh_feature_caches,
                self.revision,
            );
        }
    }

    pub(crate) fn sync_streaming_region(&mut self, _center: IVec3) {
        // Cold-cache readiness is queried directly from the shared OnceLocks.
    }

    pub(crate) fn revision(&self) -> TaskInputRevision {
        self.revision
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn structure_top_chunk_if_ready(&self, horizontal: IVec2) -> Option<i32> {
        self.snapshot
            .as_ref()?
            .structure_top_chunk_if_ready(horizontal)
    }

    pub(crate) fn contains(&self, coord: IVec3) -> bool {
        self.pending.contains(ChunkCoord::from_ivec3(coord))
    }

    pub(crate) fn schedule(&mut self, coord: IVec3, limiter: &ChunkAsyncWorkLimiter) -> bool {
        self.schedule_with_permit(
            ChunkCoord::from_ivec3(coord),
            MAX_GENERATION_TASKS_IN_FLIGHT,
            || limiter.try_acquire_generation(),
        )
    }

    pub(crate) fn schedule_loading(
        &mut self,
        coord: IVec3,
        limiter: &ChunkAsyncWorkLimiter,
    ) -> bool {
        self.schedule_with_permit(
            ChunkCoord::from_ivec3(coord),
            limiter.loading_queue_limit(),
            || limiter.try_acquire_loading_generation(),
        )
    }

    fn schedule_with_permit(
        &mut self,
        coord: ChunkCoord,
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
            .unwrap_or_else(|| {
                panic!("chunk generation snapshot must be prepared before scheduling")
            })
            .clone();
        let revision = self.revision;
        let job = ChunkGenerationJob::new(coord, snapshot);
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let _permit = permit;
            job.run()
        });

        self.pending.insert(coord, revision, task)
    }

    pub(crate) fn cancel_where(&mut self, mut predicate: impl FnMut(IVec3) -> bool) -> Vec<IVec3> {
        let started = Instant::now();
        let cancelled = self
            .pending
            .cancel_where(|coord| predicate(coord.as_ivec3()))
            .into_iter()
            .map(ChunkCoord::as_ivec3)
            .collect::<Vec<_>>();
        let elapsed = started.elapsed();
        if elapsed >= SLOW_GENERATION_SCHEDULER_WARNING {
            warn!(
                "slow streaming generation task cancellation: cancelled={} remaining={} elapsed_us={}",
                cancelled.len(),
                self.pending.len(),
                elapsed.as_micros(),
            );
        }
        cancelled
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<VoxelChunk>> {
        self.pending
            .poll_ready()
            .map(CompletedChunkTask::into_runtime)
    }
}
