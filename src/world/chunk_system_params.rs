use bevy::{ecs::system::SystemParam, prelude::*};

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry, layer::LayerRegistry,
    secondary_property::SecondaryPropertyRegistry,
};

use super::chunk_rendering::{ChunkRenderContext, ChunkRenderPool, FluidMaterials, TerrainMaterials};

#[derive(SystemParam)]
pub(crate) struct VoxelContent<'w> {
    pub(crate) blocks: Res<'w, BlockRegistry>,
    pub(crate) layers: Res<'w, LayerRegistry>,
    pub(crate) fluids: Res<'w, FluidRegistry>,
    pub(crate) secondary_properties: Res<'w, SecondaryPropertyRegistry>,
}

#[derive(SystemParam)]
pub(crate) struct ChunkContent<'w> {
    voxel: VoxelContent<'w>,
}

impl ChunkContent<'_> {
    pub(crate) fn blocks(&self) -> &BlockRegistry {
        &self.voxel.blocks
    }

    pub(crate) fn layers(&self) -> &LayerRegistry {
        &self.voxel.layers
    }

    pub(crate) fn fluids(&self) -> &FluidRegistry {
        &self.voxel.fluids
    }

    pub(crate) fn secondary_properties(&self) -> &SecondaryPropertyRegistry {
        &self.voxel.secondary_properties
    }

    pub(crate) fn mesh_inputs_changed(&self) -> bool {
        self.voxel.blocks.is_changed()
            || self.voxel.layers.is_changed()
            || self.voxel.fluids.is_changed()
            || self.voxel.secondary_properties.is_changed()
    }

    pub(crate) fn render_context<'a>(
        &'a self,
        world: &'a crate::voxel::world::VoxelWorld,
        terrain_materials: &'a TerrainMaterials,
        fluid_materials: &'a FluidMaterials,
    ) -> ChunkRenderContext<'a> {
        ChunkRenderContext {
            world,
            blocks: self.blocks(),
            layers: self.layers(),
            fluids: self.fluids(),
            secondary_properties: self.secondary_properties(),
            terrain_materials,
            fluid_materials,
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
