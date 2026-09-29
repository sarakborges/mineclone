use std::sync::Arc;

use crate::voxel::{chunk::VoxelChunk, coordinates::ChunkCoord};

use super::{generation::generate_chunk, generation_snapshot::GenerationSnapshot};

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
}

#[cfg(test)]
mod tests {
    use bevy::prelude::IVec3;

    use super::*;
    use crate::{
        content::read_content,
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

    #[test]
    fn same_snapshot_and_coord_produce_identical_authoritative_content() {
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
        let feature_fields = WorldFeatureFields::new(TEST_SEED).with_structure_field(
            StructureField::from_content(
                TEST_SEED,
                &content.biomes,
                &content.structures,
                &content.structure_sets,
            ),
        );
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
        let coord = ChunkCoord::from_ivec3(IVec3::new(0, 2, 0));

        let first = ChunkGenerationJob::new(coord, Arc::clone(&snapshot)).run();
        let second = ChunkGenerationJob::new(coord, snapshot).run();
        let first_disk = DiskChunk::from_chunk(coord.as_ivec3(), &first, &content.fluids)
            .expect("generated chunk must serialize to authoritative disk form");
        let second_disk = DiskChunk::from_chunk(coord.as_ivec3(), &second, &content.fluids)
            .expect("generated chunk must serialize to authoritative disk form");

        assert_eq!(
            serde_json::to_vec(&first_disk).expect("first generated chunk must serialize"),
            serde_json::to_vec(&second_disk).expect("second generated chunk must serialize"),
        );
    }
}
