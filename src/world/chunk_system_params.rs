use bevy::{ecs::system::SystemParam, prelude::*};

use crate::content::{
    biome::BiomeRegistry,
    block::BlockRegistry,
    dimension::DimensionDefinition,
    fluid::FluidRegistry,
    secondary_property::SecondaryPropertyRegistry,
    structure::StructureRegistry,
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{ChunkRenderContext, ChunkRenderPool, FluidMaterials, TerrainMaterials},
    current_context::CurrentDimensionContext,
    generation::ChunkGenerationContext,
    world_feature_fields::WorldFeatureFields,
};

#[derive(SystemParam)]
pub(crate) struct VoxelContent<'w> {
    pub(crate) blocks: Res<'w, BlockRegistry>,
    pub(crate) fluids: Res<'w, FluidRegistry>,
    pub(crate) secondary_properties: Res<'w, SecondaryPropertyRegistry>,
}

#[derive(SystemParam)]
pub(crate) struct ChunkContent<'w> {
    pub(crate) blocks: Res<'w, BlockRegistry>,
    pub(crate) fluids: Res<'w, FluidRegistry>,
    pub(crate) biomes: Res<'w, BiomeRegistry>,
    pub(crate) secondary_properties: Res<'w, SecondaryPropertyRegistry>,
    pub(crate) biome_field: Res<'w, BiomeField>,
}

impl<'w> ChunkContent<'w> {
    pub(crate) fn render_context<'a>(
        &'a self,
        world: &'a crate::voxel::world::VoxelWorld,
        terrain_materials: &'a TerrainMaterials,
        fluid_materials: &'a FluidMaterials,
    ) -> ChunkRenderContext<'a> {
        ChunkRenderContext {
            world,
            blocks: &self.blocks,
            fluids: &self.fluids,
            biomes: &self.biomes,
            secondary_properties: &self.secondary_properties,
            biome_field: &self.biome_field,
            terrain_materials,
            fluid_materials,
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct ChunkGeneration<'w> {
    pub(crate) dimension: CurrentDimensionContext<'w>,
    pub(crate) structures: Res<'w, StructureRegistry>,
    pub(crate) feature_fields: Res<'w, WorldFeatureFields>,
}

impl<'w> ChunkGeneration<'w> {
    pub(crate) fn dimension(&self) -> &DimensionDefinition {
        self.dimension.definition().unwrap_or_else(|| {
            panic!("missing dimension definition: {}", self.dimension.id())
        })
    }

    pub(crate) fn context<'a>(&'a self, content: &'a ChunkContent<'_>) -> ChunkGenerationContext<'a> {
        ChunkGenerationContext {
            blocks: &content.blocks,
            fluids: &content.fluids,
            dimension: self.dimension(),
            biomes: &content.biomes,
            structures: &self.structures,
            biome_field: &content.biome_field,
            feature_fields: &self.feature_fields,
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct ChunkRenderer<'w, 's> {
    pub(crate) commands: Commands<'w, 's>,
    pub(crate) meshes: ResMut<'w, Assets<Mesh>>,
    pub(crate) pool: ResMut<'w, ChunkRenderPool>,
    pub(crate) terrain_materials: Res<'w, TerrainMaterials>,
    pub(crate) fluid_materials: Res<'w, FluidMaterials>,
}
