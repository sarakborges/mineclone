use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry,
        layer::LayerRegistry, secondary_property::SecondaryPropertyRegistry,
    },
    rendering::block_texture::TerrainTextureTable,
    voxel::mesh_snapshot::ChunkMeshSnapshot,
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::ChunkMeshBuildContext,
    chunk_system_params::ChunkContent,
};

/// Immutable authored/content inputs shared by background voxel presentation
/// jobs. Mesh and remesh schedulers decide when to execute; this type owns the
/// presentation input boundary they are allowed to capture.
pub(crate) struct PresentationContentSnapshot {
    blocks: BlockRegistry,
    layers: LayerRegistry,
    fluids: FluidRegistry,
    biomes: BiomeRegistry,
    secondary_properties: SecondaryPropertyRegistry,
    biome_field: BiomeField,
    texture_table: TerrainTextureTable,
}

impl PresentationContentSnapshot {
    pub(crate) fn capture(content: &ChunkContent<'_>) -> Self {
        Self {
            blocks: content.blocks().clone(),
            layers: content.layers().clone(),
            fluids: content.fluids().clone(),
            biomes: BiomeRegistry::clone(&content.biomes),
            secondary_properties: content.secondary_properties().clone(),
            biome_field: content.biome_field.as_ref().clone(),
            texture_table: TerrainTextureTable::from_blocks(content.blocks()),
        }
    }

    pub(crate) fn context<'a>(
        &'a self,
        world: &'a ChunkMeshSnapshot,
    ) -> ChunkMeshBuildContext<'a, ChunkMeshSnapshot> {
        ChunkMeshBuildContext {
            world,
            blocks: &self.blocks,
            layers: &self.layers,
            fluids: &self.fluids,
            biomes: &self.biomes,
            secondary_properties: &self.secondary_properties,
            biome_field: &self.biome_field,
            texture_table: &self.texture_table,
        }
    }
}
