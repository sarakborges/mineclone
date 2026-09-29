use bevy::prelude::IVec3;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry,
        layer::LayerRegistry, secondary_property::SecondaryPropertyRegistry,
    },
    rendering::block_texture::TerrainTextureTable,
    voxel::{
        coordinates::ChunkCoord,
        mesh_snapshot::{ChunkMeshDependencies, ChunkMeshSnapshot, ChunkSnapshotSource},
        meshlet::ChunkMeshletMask,
    },
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::ChunkMeshBuildContext,
    chunk_system_params::ChunkContent,
};

/// Immutable world-content identity captured by a presentation job. This is
/// intentionally separate from authored/content-definition revisions and from
/// lighting revisions, which have independent owners.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChunkPresentationSource {
    coord: ChunkCoord,
    revisions: ChunkMeshDependencies,
}

impl ChunkPresentationSource {
    pub(crate) fn capture(coord: ChunkCoord, world: &ChunkMeshSnapshot) -> Self {
        Self {
            coord,
            revisions: world.dependencies(),
        }
    }

    pub(crate) fn for_meshlets(mut self, meshlets: ChunkMeshletMask) -> Self {
        self.revisions = self.revisions.for_meshlets(meshlets);
        self
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

    /// Temporary compatibility shim for remesh scheduling while call sites are
    /// migrated onto the presentation-owned snapshot vocabulary.
    pub(crate) fn from_content(content: &ChunkContent<'_>) -> Self {
        Self::capture(content)
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
