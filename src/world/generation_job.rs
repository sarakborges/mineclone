use std::sync::Arc;

use crate::voxel::{chunk::VoxelChunk, coordinates::ChunkCoord};

use super::{
    generation::{ChunkGenerationPassTimings, generate_chunk, generate_chunk_profiled},
    generation_snapshot::GenerationSnapshot,
};

/// Immutable, scheduler-independent unit of chunk generation work.
///
/// Scheduling policy owns when and where this job executes. The job owns only
/// the deterministic calculation inputs required to produce authoritative
/// chunk data; it has no ECS, rendering, UI, or publication side effects.
pub(crate) struct ChunkGenerationJob {
    coord: ChunkCoord,
    snapshot: Arc<GenerationSnapshot>,
}

impl ChunkGenerationJob {
    pub(crate) fn new(coord: ChunkCoord, snapshot: Arc<GenerationSnapshot>) -> Self {
        Self { coord, snapshot }
    }

    pub(crate) fn run(self) -> VoxelChunk {
        let context = self.snapshot.context();
        generate_chunk(self.coord.as_ivec3(), &context)
    }

    pub(crate) fn run_profiled(self) -> (VoxelChunk, ChunkGenerationPassTimings) {
        let context = self.snapshot.context();
        generate_chunk_profiled(self.coord.as_ivec3(), &context)
    }
}

#[cfg(test)]
mod tests {
    use std::{hint::black_box, time::Instant};

    use bevy::prelude::IVec3;

    use super::*;
    use crate::{
        content::{fluid::FluidRegistry, read_content},
        voxel::chunk_disk::DiskChunk,
        world::{
            biome_field::BiomeField,
            generation::ChunkGenerationContext,
            new_world::{DEFAULT_BIOME_SIZE_MULTIPLIER, WorldGenerationSettings},
            structure_field::StructureField,
            world_feature_fields::WorldFeatureFields,
        },
    };

    const TEST_SEED: u64 = 0x7d4a_21c8_95e3_b60f;
    const TEST_DIMENSION: &str = "asteria:overworld";

    fn test_snapshot() -> (Arc<GenerationSnapshot>, FluidRegistry) {
        let content = read_content();
        let dimension = content
            .dimensions
            .get(TEST_DIMENSION)
            .unwrap_or_else(|| panic!("missing test dimension: {TEST_DIMENSION}"));
        let world_generation = WorldGenerationSettings::default();
        let mut biome_field = BiomeField::from_dimension(
            dimension,
            &content.biomes,
            TEST_SEED,
            DEFAULT_BIOME_SIZE_MULTIPLIER,
        );
        biome_field.set_spawn_oceans(world_generation.spawn_oceans());
        let feature_fields =
            WorldFeatureFields::new(TEST_SEED).with_structure_field(StructureField::from_content(
                TEST_SEED,
                &content.biomes,
                &content.structures,
                &content.structure_sets,
            ));
        let context = ChunkGenerationContext {
            blocks: &content.blocks,
            fluids: &content.fluids,
            dimension,
            biomes: &content.biomes,
            structures: &content.structures,
            structure_sets: &content.structure_sets,
            world_generation,
            biome_field: &biome_field,
            feature_fields: &feature_fields,
        };
        let snapshot = Arc::new(GenerationSnapshot::from_context(&context, false));

        (snapshot, content.fluids.clone())
    }

    #[test]
    fn same_snapshot_and_coord_produce_identical_authoritative_content() {
        let (snapshot, fluids) = test_snapshot();
        let coord = ChunkCoord::from_ivec3(IVec3::new(0, 2, 0));

        let first = ChunkGenerationJob::new(coord, Arc::clone(&snapshot)).run();
        let second = ChunkGenerationJob::new(coord, snapshot).run();
        let first_disk = DiskChunk::from_chunk(coord.as_ivec3(), &first, &fluids)
            .expect("generated chunk must serialize to authoritative disk form");
        let second_disk = DiskChunk::from_chunk(coord.as_ivec3(), &second, &fluids)
            .expect("generated chunk must serialize to authoritative disk form");

        assert_eq!(
            serde_json::to_vec(&first_disk).expect("first generated chunk must serialize"),
            serde_json::to_vec(&second_disk).expect("second generated chunk must serialize"),
        );
    }

    /// Manual job-only benchmark. Keep setup and cache warm-up outside the
    /// measured region so this does not accidentally become a scheduler,
    /// content-loading, or render benchmark.
    ///
    /// Run with:
    /// `cargo test --release --locked benchmark_chunk_generation_job -- --ignored --nocapture --test-threads=1`
    #[test]
    #[ignore]
    fn benchmark_chunk_generation_job() {
        let (snapshot, _) = test_snapshot();
        let coords = [
            ChunkCoord::new(0, 2, 0),
            ChunkCoord::new(1, 2, 0),
            ChunkCoord::new(-1, 2, 0),
            ChunkCoord::new(0, 2, 1),
            ChunkCoord::new(0, 2, -1),
            ChunkCoord::new(1, 2, 1),
            ChunkCoord::new(-1, 2, -1),
            ChunkCoord::new(2, 2, 0),
        ];

        for &coord in &coords {
            black_box(ChunkGenerationJob::new(coord, Arc::clone(&snapshot)).run());
        }

        let mut samples_ms = Vec::with_capacity(coords.len());
        for &coord in &coords {
            let started = Instant::now();
            black_box(ChunkGenerationJob::new(coord, Arc::clone(&snapshot)).run());
            samples_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        samples_ms.sort_by(f64::total_cmp);

        let average_ms = samples_ms.iter().sum::<f64>() / samples_ms.len() as f64;
        let p50_ms = samples_ms[samples_ms.len() / 2];
        let max_ms = *samples_ms
            .last()
            .expect("worldgen benchmark must record at least one sample");
        println!(
            "chunk_generation_job warm benchmark: samples={} avg_ms={average_ms:.3} p50_ms={p50_ms:.3} max_ms={max_ms:.3}",
            samples_ms.len(),
        );
    }
}
