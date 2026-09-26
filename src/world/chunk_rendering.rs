mod materials;
mod pool;
mod refresh;
mod spawn;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry,
        layer::LayerRegistry, secondary_property::SecondaryPropertyRegistry,
    },
    rendering::block_texture::TerrainTextureTable,
    voxel::{
        chunk::VoxelChunk, fluid_mesh::ChunkFluidMesh, meshlet::ChunkMeshletMask,
        read::VoxelRead, world::VoxelWorld,
    },
};

use super::biome_field::BiomeField;

pub(crate) use materials::{FluidMaterials, TerrainMaterials};
pub(crate) use pool::{
    ChunkRenderPool, DeferredMeshAssetRetirements, advance_deferred_mesh_asset_retirements,
    clear_chunk_render_pool, retire_chunk_render_allocation,
};
pub(crate) use refresh::{
    apply_built_chunk_fluid_meshlets, apply_built_chunk_geometry_meshlets,
};
pub(crate) use spawn::{BuiltChunkMesh, build_chunk_render_meshes, spawn_built_chunk_meshes};

const MEBIBYTE: usize = 1024 * 1024;

/// Chunk meshes are only one consumer of VRAM. Keep the steady-state pool well
/// below the allocator's historical 256 MiB envelope so Bevy can grow its
/// general vertex/index slabs without needing the old and new buffers to fit at
/// the absolute residency limit.
const CHUNK_MESH_RESIDENCY_HIGH_BYTES: usize = 192 * MEBIBYTE;
const CHUNK_MESH_RESIDENCY_TARGET_BYTES: usize = 160 * MEBIBYTE;
const CHUNK_MESH_RESIDENCY_RECOVERY_BYTES: usize = 128 * MEBIBYTE;

pub(crate) fn chunk_mesh_residency_high_bytes(_render_distance_chunks: i32) -> usize {
    CHUNK_MESH_RESIDENCY_HIGH_BYTES
}

pub(crate) fn chunk_mesh_residency_target_bytes(_render_distance_chunks: i32) -> usize {
    CHUNK_MESH_RESIDENCY_TARGET_BYTES
}

pub(crate) fn chunk_mesh_residency_recovery_bytes(_render_distance_chunks: i32) -> usize {
    CHUNK_MESH_RESIDENCY_RECOVERY_BYTES
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChunkRenderCoord(pub(crate) IVec3);

pub(crate) struct ChunkMeshBuildContext<'a, W: VoxelRead + ?Sized> {
    pub(crate) world: &'a W,
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) layers: &'a LayerRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) secondary_properties: &'a SecondaryPropertyRegistry,
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) texture_table: &'a TerrainTextureTable,
}

pub(crate) struct ChunkRenderContext<'a> {
    pub(crate) world: &'a VoxelWorld,
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) layers: &'a LayerRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) secondary_properties: &'a SecondaryPropertyRegistry,
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) terrain_materials: &'a TerrainMaterials,
    pub(crate) fluid_materials: &'a FluidMaterials,
}

impl ChunkRenderContext<'_> {
    pub(crate) fn mesh_build_context(&self) -> ChunkMeshBuildContext<'_, VoxelWorld> {
        ChunkMeshBuildContext {
            world: self.world,
            blocks: self.blocks,
            layers: self.layers,
            fluids: self.fluids,
            biomes: self.biomes,
            secondary_properties: self.secondary_properties,
            biome_field: self.biome_field,
            texture_table: self.terrain_materials.texture_table(),
        }
    }
}

pub(crate) fn build_chunk_terrain_meshlet_remeshes<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
    meshlets: ChunkMeshletMask,
) -> Vec<BuiltChunkMesh> {
    spawn::build_chunk_terrain_render_meshlets(coord, chunk, context, meshlets)
}

pub(crate) fn build_chunk_fluid_meshlet_remeshes<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
    meshlets: ChunkMeshletMask,
) -> Vec<ChunkFluidMesh> {
    spawn::build_chunk_fluid_render_meshlets(coord, chunk, context, meshlets)
}
