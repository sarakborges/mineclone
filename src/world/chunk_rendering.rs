mod materials;
mod pool;
mod refresh;
mod spawn;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry,
        secondary_property::SecondaryPropertyRegistry,
    },
    voxel::{read::VoxelRead, world::VoxelWorld},
};

use super::biome_field::BiomeField;

pub(crate) use materials::{FluidMaterials, TerrainMaterials};
pub(crate) use pool::{
    ChunkRenderPool, clear_chunk_render_pool, retire_chunk_render_allocation,
};
pub(crate) use refresh::{refresh_chunk_fluid_mesh, refresh_chunk_geometry_mesh};
pub(crate) use spawn::{
    BuiltChunkMesh, build_chunk_render_meshes, spawn_built_chunk_meshes, spawn_chunk_mesh,
};

pub(crate) struct ChunkMeshBuildContext<'a, W: VoxelRead + ?Sized> {
    pub(crate) world: &'a W,
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) secondary_properties: &'a SecondaryPropertyRegistry,
    pub(crate) biome_field: &'a BiomeField,
}

pub(crate) struct ChunkRenderContext<'a> {
    pub(crate) world: &'a VoxelWorld,
    pub(crate) blocks: &'a BlockRegistry,
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
            fluids: self.fluids,
            biomes: self.biomes,
            secondary_properties: self.secondary_properties,
            biome_field: self.biome_field,
        }
    }
}
