use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::voxel::{chunk::VoxelChunk, coordinates::ChunkCoord};

use super::{
    chunk_async_work::{ChunkAsyncWorkLimiter, ChunkAsyncWorkPermit},
    chunk_system_params::{ChunkContent, ChunkGeneration},
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
    generation::ChunkGenerationPassTimings,
    generation_job::ChunkGenerationJob,
    generation_snapshot::GenerationSnapshot,
    revision::TaskInputRevision,
};

pub(crate) const MAX_GENERATION_TASKS_IN_FLIGHT: usize = 8;
const SLOW_GENERATION_SCHEDULER_WARNING: Duration = Duration::from_millis(8);

#[derive(Default)]
struct GenerationStageMetrics {
    count: AtomicU64,
    total_nanos: AtomicU64,
    max_nanos: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GenerationStageDiagnostic {
    count: u64,
    average_micros: u64,
    max_micros: u64,
}

impl fmt::Display for GenerationStageDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "count:{} avg_us:{} max_us:{}",
            self.count, self.average_micros, self.max_micros
        )
    }
}

impl GenerationStageMetrics {
    fn record(&self, elapsed: Option<Duration>) {
        let Some(elapsed) = elapsed else {
            return;
        };
        let elapsed_nanos = elapsed.as_nanos().min(u128::from(u64::MAX)) as u64;
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_nanos
            .fetch_add(elapsed_nanos, Ordering::Relaxed);
        self.max_nanos.fetch_max(elapsed_nanos, Ordering::Relaxed);
    }

    fn diagnostic(&self) -> GenerationStageDiagnostic {
        let count = self.count.load(Ordering::Relaxed);
        let total_nanos = self.total_nanos.load(Ordering::Relaxed);
        let max_nanos = self.max_nanos.load(Ordering::Relaxed);
        GenerationStageDiagnostic {
            count,
            average_micros: total_nanos.checked_div(count).unwrap_or(0) / 1_000,
            max_micros: max_nanos / 1_000,
        }
    }
}

#[derive(Default)]
struct GenerationPipelineMetrics {
    biome_map: GenerationStageMetrics,
    structure_extent: GenerationStageMetrics,
    terrain_columns: GenerationStageMetrics,
    volume_biomes: GenerationStageMetrics,
    density_field: GenerationStageMetrics,
    materials: GenerationStageMetrics,
    initial_fluids: GenerationStageMetrics,
    structures: GenerationStageMetrics,
    surface_objects: GenerationStageMetrics,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GenerationPipelineDiagnostic {
    biome_map: GenerationStageDiagnostic,
    structure_extent: GenerationStageDiagnostic,
    terrain_columns: GenerationStageDiagnostic,
    volume_biomes: GenerationStageDiagnostic,
    density_field: GenerationStageDiagnostic,
    materials: GenerationStageDiagnostic,
    initial_fluids: GenerationStageDiagnostic,
    structures: GenerationStageDiagnostic,
    surface_objects: GenerationStageDiagnostic,
}

impl fmt::Display for GenerationPipelineDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "biome_map[{}] structure_extent[{}] terrain_columns[{}] volume_biomes[{}] density_field[{}] materials[{}] initial_fluids[{}] structures[{}] surface_objects[{}]",
            self.biome_map,
            self.structure_extent,
            self.terrain_columns,
            self.volume_biomes,
            self.density_field,
            self.materials,
            self.initial_fluids,
            self.structures,
            self.surface_objects,
        )
    }
}

impl GenerationPipelineMetrics {
    fn record(&self, timings: ChunkGenerationPassTimings) {
        self.biome_map.record(timings.biome_map);
        self.structure_extent.record(timings.structure_extent);
        self.terrain_columns.record(timings.terrain_columns);
        self.volume_biomes.record(timings.volume_biomes);
        self.density_field.record(timings.density_field);
        self.materials.record(timings.materials);
        self.initial_fluids.record(timings.initial_fluids);
        self.structures.record(timings.structures);
        self.surface_objects.record(timings.surface_objects);
    }

    fn diagnostic(&self) -> GenerationPipelineDiagnostic {
        GenerationPipelineDiagnostic {
            biome_map: self.biome_map.diagnostic(),
            structure_extent: self.structure_extent.diagnostic(),
            terrain_columns: self.terrain_columns.diagnostic(),
            volume_biomes: self.volume_biomes.diagnostic(),
            density_field: self.density_field.diagnostic(),
            materials: self.materials.diagnostic(),
            initial_fluids: self.initial_fluids.diagnostic(),
            structures: self.structures.diagnostic(),
            surface_objects: self.surface_objects.diagnostic(),
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct GenerationScheduler {
    revision: TaskInputRevision,
    snapshot: Option<Arc<GenerationSnapshot>>,
    pending: ChunkTaskQueue<VoxelChunk>,
    metrics: Arc<GenerationPipelineMetrics>,
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
        self.metrics = Arc::new(GenerationPipelineMetrics::default());
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

    pub(crate) fn diagnostics(&self) -> GenerationPipelineDiagnostic {
        self.metrics.diagnostic()
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
        let metrics = Arc::clone(&self.metrics);
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let _permit = permit;
            let (chunk, timings) = job.run_profiled();
            metrics.record(timings);
            chunk
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_pipeline_metrics_aggregate_counts_average_and_max() {
        let metrics = GenerationPipelineMetrics::default();
        metrics.record(ChunkGenerationPassTimings {
            biome_map: Some(Duration::from_micros(10)),
            density_field: Some(Duration::from_micros(30)),
            ..ChunkGenerationPassTimings::default()
        });
        metrics.record(ChunkGenerationPassTimings {
            biome_map: Some(Duration::from_micros(20)),
            density_field: Some(Duration::from_micros(50)),
            ..ChunkGenerationPassTimings::default()
        });

        let diagnostic = metrics.diagnostic();
        assert_eq!(diagnostic.biome_map.count, 2);
        assert_eq!(diagnostic.biome_map.average_micros, 15);
        assert_eq!(diagnostic.biome_map.max_micros, 20);
        assert_eq!(diagnostic.density_field.count, 2);
        assert_eq!(diagnostic.density_field.average_micros, 40);
        assert_eq!(diagnostic.density_field.max_micros, 50);
        assert_eq!(diagnostic.structures.count, 0);
    }
}
