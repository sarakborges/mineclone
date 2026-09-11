use bevy::{ecs::system::SystemParam, prelude::*};

use crate::content::{
    biome::BiomeRegistry,
    block::BlockRegistry,
    dimension::{DimensionDefinition, DimensionRegistry},
    fluid::FluidRegistry,
    structure::StructureRegistry,
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{ChunkRenderContext, ChunkRenderPool, FluidMaterials, TerrainMaterials},
    dimension::CurrentDimension,
    generation::ChunkGenerationContext,
    world_feature_fields::WorldFeatureFields,
};

#[derive(SystemParam)]
pub(crate) struct ChunkContent<'w> {
    pub blocks: Res<'w, BlockRegistry>,
    pub fluids: Res<'w, FluidRegistry>,
    pub biomes: Res<'w, BiomeRegistry>,
    pub structures: Res<'w, StructureRegistry>,
    pub biome_field: Res<'w, BiomeField>,
}

impl<'w> ChunkContent<'w> {
    pub fn render_context<'a>(
        &'a self,
        world: &'a crate::voxel::world::VoxelWorld,
        terrain_materials: &'a TerrainMaterials,
        fluid_materials: &'a FluidMaterials,
    ) -> ChunkRenderContext<'a> {
        ChunkRenderContext {
            world,
            blocks: &self.blocks,
            biomes: &self.biomes,
            biome_field: &self.biome_field,
            terrain_materials,
            fluid_materials,
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct ChunkGeneration<'w> {
    pub current_dimension: Res<'w, CurrentDimension>,
    pub dimensions: Res<'w, DimensionRegistry>,
    pub feature_fields: Res<'w, WorldFeatureFields>,
}

impl<'w> ChunkGeneration<'w> {
    pub fn dimension(&self) -> &DimensionDefinition {
        self.dimensions
            .get(&self.current_dimension.id)
            .unwrap_or_else(|| {
                panic!(
                    "missing dimension definition: {}",
                    self.current_dimension.id
                )
            })
    }

    pub fn context<'a>(&'a self, content: &'a ChunkContent<'_>) -> ChunkGenerationContext<'a> {
        ChunkGenerationContext {
            blocks: &content.blocks,
            fluids: &content.fluids,
            dimension: self.dimension(),
            biomes: &content.biomes,
            structures: &content.structures,
            biome_field: &content.biome_field,
            feature_fields: &self.feature_fields,
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct ChunkRenderer<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub pool: ResMut<'w, ChunkRenderPool>,
    pub terrain_materials: Res<'w, TerrainMaterials>,
    pub fluid_materials: Res<'w, FluidMaterials>,
}
