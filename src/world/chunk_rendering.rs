mod materials;
mod pool;
mod refresh;
mod spawn;

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry},
    voxel::world::VoxelWorld,
};

use super::biome_field::BiomeField;

pub use materials::{FluidMaterials, TerrainMaterials};
pub use pool::{clear_chunk_render_pool, ChunkRenderPool};
pub use refresh::{refresh_adjacent_chunk_meshes, refresh_chunk_mesh};
pub use spawn::spawn_chunk_mesh;

pub(crate) struct ChunkRenderContext<'a> {
    pub world: &'a VoxelWorld,
    pub blocks: &'a BlockRegistry,
    pub biomes: &'a BiomeRegistry,
    pub biome_field: &'a BiomeField,
    pub terrain_materials: &'a TerrainMaterials,
    pub fluid_materials: &'a FluidMaterials,
}
