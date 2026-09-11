mod materials;
mod pool;
mod refresh;
mod spawn;

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry},
    voxel::world::VoxelWorld,
};

use super::biome_field::BiomeField;

pub(crate) use materials::{FluidMaterials, TerrainMaterials};
pub(crate) use pool::{ChunkRenderPool, clear_chunk_render_pool};
pub(crate) use refresh::refresh_chunk_mesh;
pub(crate) use spawn::spawn_chunk_mesh;

pub(crate) struct ChunkRenderContext<'a> {
    pub(crate) world: &'a VoxelWorld,
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) terrain_materials: &'a TerrainMaterials,
    pub(crate) fluid_materials: &'a FluidMaterials,
}
