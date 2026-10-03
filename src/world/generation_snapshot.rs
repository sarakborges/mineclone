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
        let context = ChunkGenerationContext {
            blocks: content.blocks(),
            fluids: content.fluids(),
            dimension: generation.dimension(),
            biomes: &content.biomes,
            structures: &generation.structures,
            structure_sets: &generation.structure_sets,
            world_generation: *generation.world_generation,
            biome_field: &content.biome_field,
            feature_fields: &generation.feature_fields,
        };
        Self::from_context(&context, fresh_feature_caches)
    }

    /// Captures the generator's domain inputs without requiring Bevy
    /// `SystemParam` wrappers. Runtime systems use `capture`; tests and
    /// benchmarks can build the same immutable job input directly.
    pub(crate) fn from_context(
        context: &ChunkGenerationContext<'_>,
        fresh_feature_caches: bool,
    ) -> Self {
        Self {
            blocks: context.blocks.clone(),
            fluids: context.fluids.clone(),
            dimension: context.dimension.clone(),
            biomes: context.biomes.clone(),
            structures: context.structures.clone(),
            structure_sets: context.structure_sets.clone(),
            world_generation: context.world_generation,
            biome_field: context.biome_field.clone(),
            feature_fields: if fresh_feature_caches {
                context.feature_fields.clone_with_fresh_caches()
            } else {
                context.feature_fields.clone()
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

    pub(crate) fn structure_top_chunk_if_ready(
        &self,
        horizontal: bevy::prelude::IVec2,
    ) -> Option<i32> {
        self.feature_fields
            .structure_top_y_if_ready(horizontal)
            .map(|top_y| top_y.div_euclid(crate::voxel::chunk::CHUNK_SIZE as i32))
    }
}
