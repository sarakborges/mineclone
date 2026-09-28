use crate::content::{
    biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionDefinition,
    fluid::FluidRegistry, structure::StructureRegistry, structure_set::StructureSetRegistry,
};

use super::{
    biome_field::BiomeField,
    chunk_system_params::{ChunkContent, ChunkGeneration},
    generation::ChunkGenerationContext,
    new_world::WorldGenerationSettings,
    world_feature_fields::WorldFeatureFields,
};

/// Immutable inputs captured for background chunk generation.
///
/// The generation scheduler owns when work runs. This type owns which data is
/// safe for that work to observe, so scheduling policy cannot accidentally
/// widen the async dependency surface.
pub(crate) struct GenerationSnapshot {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    dimension: DimensionDefinition,
    biomes: BiomeRegistry,
    structures: StructureRegistry,
    structure_sets: StructureSetRegistry,
    world_generation: WorldGenerationSettings,
    biome_field: BiomeField,
    feature_fields: WorldFeatureFields,
}

impl GenerationSnapshot {
    pub(crate) fn capture(
        generation: &ChunkGeneration<'_>,
        content: &ChunkContent<'_>,
        fresh_feature_caches: bool,
    ) -> Self {
        Self {
            blocks: content.blocks().clone(),
            fluids: content.fluids().clone(),
            dimension: generation.dimension().clone(),
            biomes: BiomeRegistry::clone(&content.biomes),
            structures: StructureRegistry::clone(&generation.structures),
            structure_sets: StructureSetRegistry::clone(&generation.structure_sets),
            world_generation: *generation.world_generation,
            biome_field: content.biome_field.as_ref().clone(),
            feature_fields: if fresh_feature_caches {
                generation.feature_fields.clone_with_fresh_caches()
            } else {
                generation.feature_fields.as_ref().clone()
            },
        }
    }

    pub(crate) fn context(&self) -> ChunkGenerationContext<'_> {
        ChunkGenerationContext {
            blocks: &self.blocks,
            fluids: &self.fluids,
            dimension: &self.dimension,
            biomes: &self.biomes,
            structures: &self.structures,
            structure_sets: &self.structure_sets,
            world_generation: self.world_generation,
            biome_field: &self.biome_field,
            feature_fields: &self.feature_fields,
        }
    }

    pub(crate) fn structure_top_chunk_if_ready(&self, horizontal: bevy::prelude::IVec2) -> Option<i32> {
        self.feature_fields
            .structure_top_y_if_ready(horizontal)
            .map(|top_y| top_y.div_euclid(crate::voxel::chunk::CHUNK_SIZE as i32))
    }
}
