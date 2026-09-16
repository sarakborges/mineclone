use std::sync::Arc;

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionDefinition,
        fluid::FluidRegistry, structure::StructureRegistry,
    },
    voxel::chunk::VoxelChunk,
};

use super::{
    biome_field::BiomeField,
    chunk_system_params::{ChunkContent, ChunkGeneration},
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
    generation::{ChunkGenerationContext, generate_chunk},
    world_feature_fields::WorldFeatureFields,
};

pub(crate) const MAX_GENERATION_TASKS_IN_FLIGHT: usize = 8;

struct GenerationSnapshot {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    dimension: DimensionDefinition,
    biomes: BiomeRegistry,
    structures: StructureRegistry,
    biome_field: BiomeField,
    feature_fields: WorldFeatureFields,
}

impl GenerationSnapshot {
    fn from_sources(generation: &ChunkGeneration<'_>, content: &ChunkContent<'_>) -> Self {
        Self {
            blocks: content.blocks().clone(),
            fluids: content.fluids().clone(),
            dimension: generation.dimension().clone(),
            biomes: BiomeRegistry::clone(&content.biomes),
            structures: StructureRegistry::clone(&generation.structures),
            biome_field: content.biome_field.as_ref().clone(),
            feature_fields: generation.feature_fields.as_ref().clone(),
        }
    }

    fn context(&self) -> ChunkGenerationContext<'_> {
        ChunkGenerationContext {
            blocks: &self.blocks,
            fluids: &self.fluids,
            dimension: &self.dimension,
            biomes: &self.biomes,
            structures: &self.structures,
            biome_field: &self.biome_field,
            feature_fields: &self.feature_fields,
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct ChunkGenerationTasks {
    revision: u64,
    snapshot: Option<Arc<GenerationSnapshot>>,
    pending: ChunkTaskQueue<VoxelChunk>,
}

impl ChunkGenerationTasks {
    pub(crate) fn sync_snapshot(
        &mut self,
        generation: &ChunkGeneration<'_>,
        content: &ChunkContent<'_>,
    ) {
        let inputs_changed = generation.inputs_changed() || content.generation_inputs_changed();
        if self.snapshot.is_some() && !inputs_changed {
            return;
        }

        self.revision = self.revision.wrapping_add(1).max(1);
        self.snapshot = Some(Arc::new(GenerationSnapshot::from_sources(
            generation, content,
        )));
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

    pub(crate) fn schedule(&mut self, coord: IVec3) -> bool {
        if self.pending.len() >= MAX_GENERATION_TASKS_IN_FLIGHT || self.pending.contains(coord) {
            return false;
        }

        let snapshot = self
            .snapshot
            .as_ref()
            .unwrap_or_else(|| panic!("chunk generation snapshot must be prepared before scheduling"))
            .clone();
        let revision = self.revision;
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let context = snapshot.context();
            generate_chunk(coord, &context)
        });

        self.pending.insert(coord, revision, task)
    }

    pub(crate) fn cancel_farthest_where(
        &mut self,
        center: IVec3,
        predicate: impl FnMut(IVec3) -> bool,
    ) -> Option<IVec3> {
        self.pending.cancel_farthest_where(center, predicate)
    }

    pub(crate) fn poll_ready(&mut self) -> Option<CompletedChunkTask<VoxelChunk>> {
        self.pending.poll_ready()
    }
}
