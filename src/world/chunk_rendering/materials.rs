use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    content::fluid::FluidId,
    rendering::{block_texture::TerrainTextureTable, terrain_material::TerrainMaterial},
    voxel::block_face::{BlockFace, BlockFaces},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TerrainAlphaKey {
    Opaque,
    Mask(u32),
    Blend,
}

/// Runtime terrain-material lookup retained by the renderer. The old loading
/// bootstrap that constructed this resource was deleted with the previous world
/// pipeline; the replacement loading path will own construction explicitly.
#[derive(Resource, Clone)]
pub struct TerrainMaterials {
    blocks: HashMap<String, BlockFaces<Vec<Handle<TerrainMaterial>>>>,
    layers: HashMap<String, Handle<TerrainMaterial>>,
    array_materials: HashMap<TerrainAlphaKey, Handle<TerrainMaterial>>,
    texture_table: TerrainTextureTable,
    texture_array: Handle<Image>,
}

impl TerrainMaterials {
    pub(crate) fn texture_table(&self) -> &TerrainTextureTable {
        &self.texture_table
    }

    pub(crate) fn texture_array_handle(&self) -> Handle<Image> {
        self.texture_array.clone()
    }

    pub(super) fn for_array(
        &self,
        alpha_blend: bool,
        alpha_cutoff: Option<u32>,
    ) -> &Handle<TerrainMaterial> {
        let alpha = if alpha_blend {
            TerrainAlphaKey::Blend
        } else if let Some(cutoff) = alpha_cutoff {
            TerrainAlphaKey::Mask(cutoff)
        } else {
            TerrainAlphaKey::Opaque
        };
        self.array_materials
            .get(&alpha)
            .unwrap_or_else(|| panic!("missing shared terrain array material for {alpha:?}"))
    }

    pub(super) fn for_face(&self, block_id: &str, face: BlockFace) -> &[Handle<TerrainMaterial>] {
        self.blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("missing terrain materials for block: {block_id}"))
            .get(face)
            .as_slice()
    }

    pub(super) fn for_layer(&self, layer_id: &str) -> &Handle<TerrainMaterial> {
        self.layers
            .get(layer_id)
            .unwrap_or_else(|| panic!("missing terrain material for layer: {layer_id}"))
    }
}

/// Runtime fluid-material lookup retained by the renderer. Construction belongs
/// to the replacement loading/bootstrap pipeline rather than legacy world code.
#[derive(Resource, Clone)]
pub struct FluidMaterials {
    materials: HashMap<FluidId, Handle<TerrainMaterial>>,
}

impl FluidMaterials {
    pub(super) fn get(&self, fluid_id: FluidId) -> &Handle<TerrainMaterial> {
        self.materials
            .get(&fluid_id)
            .unwrap_or_else(|| panic!("missing material for fluid id {fluid_id}"))
    }
}
