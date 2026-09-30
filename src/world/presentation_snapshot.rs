use bevy::{
    platform::collections::HashMap,
    prelude::{IVec3, Resource},
};

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
        revision::ChunkContentRevision,
    },
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::ChunkMeshBuildContext,
    chunk_system_params::ChunkContent,
};

/// Immutable world-content identity captured by a presentation job or
/// synchronous publication. Mesh-backed sources retain halo revisions; empty
/// synchronous publications retain only the center revision. This remains
/// separate from authored/content-definition and lighting revisions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChunkPresentationSource {
    coord: ChunkCoord,
    mesh_revisions: Option<ChunkMeshDependencies>,
    center_revision: Option<ChunkContentRevision>,
}

impl ChunkPresentationSource {
    pub(crate) fn capture(coord: ChunkCoord, world: &ChunkMeshSnapshot) -> Self {
        Self {
            coord,
            mesh_revisions: Some(world.dependencies()),
            center_revision: None,
        }
    }

    pub(crate) fn capture_center(
        coord: IVec3,
        source: &impl ChunkSnapshotSource,
    ) -> Option<Self> {
        let coord = ChunkCoord::from_ivec3(coord);
        Some(Self {
            coord,
            mesh_revisions: None,
            center_revision: Some(source.chunk_content_revision(coord)?),
        })
    }

    pub(crate) fn for_meshlets(mut self, meshlets: ChunkMeshletMask) -> Self {
        if let Some(revisions) = self.mesh_revisions {
            self.mesh_revisions = Some(revisions.for_meshlets(meshlets));
        }
        self
    }

    pub(crate) fn is_current(&self, source: &impl ChunkSnapshotSource) -> bool {
        if let Some(revisions) = self.mesh_revisions {
            return source.snapshot_chunk(self.coord).is_some() && revisions.is_current(source);
        }
        self.center_revision
            .is_some_and(|expected| source.chunk_content_revision(self.coord) == Some(expected))
    }

    pub(crate) fn initial_catchup_meshlets_with(
        &self,
        source: &impl ChunkSnapshotSource,
        neighbor_is_visible: impl FnMut(IVec3) -> bool,
    ) -> ChunkMeshletMask {
        self.mesh_revisions.map_or_else(
            ChunkMeshletMask::default,
            |revisions| revisions.initial_catchup_meshlets_with(source, neighbor_is_visible),
        )
    }
}

/// Presentation-owned lighting revision state. Revisions are section-aware so
/// a partial remesh can publish one meshlet without claiming untouched
/// meshlets observed the same lighting state.
#[derive(Resource, Clone, Default)]
pub(crate) struct PresentationLightingRevisions {
    revisions: HashMap<ChunkCoord, [u64; 8]>,
}

impl PresentationLightingRevisions {
    pub(crate) fn capture(
        &self,
        center: IVec3,
        meshlets: ChunkMeshletMask,
    ) -> PresentationLightingSource {
        let center = ChunkCoord::from_ivec3(center);
        PresentationLightingSource {
            center,
            meshlets,
            expected: self.revisions.get(&center).copied().unwrap_or([0; 8]),
        }
    }

    pub(crate) fn bump(&mut self, coord: IVec3, meshlets: ChunkMeshletMask) {
        if meshlets.is_empty() {
            return;
        }

        let entry = self
            .revisions
            .entry(ChunkCoord::from_ivec3(coord))
            .or_insert([0; 8]);
        for (index, revision) in entry.iter_mut().enumerate() {
            if meshlets.contains_index(index) {
                *revision = revision.wrapping_add(1).max(1);
            }
        }
    }

    pub(crate) fn remove(&mut self, coord: IVec3) {
        self.revisions.remove(&ChunkCoord::from_ivec3(coord));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PresentationLightingSource {
    center: ChunkCoord,
    meshlets: ChunkMeshletMask,
    expected: [u64; 8],
}

impl PresentationLightingSource {
    pub(crate) fn is_current(&self, current: &PresentationLightingRevisions) -> bool {
        let revisions = current
            .revisions
            .get(&self.center)
            .copied()
            .unwrap_or([0; 8]);
        (0..8).all(|index| {
            !self.meshlets.contains_index(index) || revisions[index] == self.expected[index]
        })
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
