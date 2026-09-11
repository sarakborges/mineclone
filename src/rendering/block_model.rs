mod geometry;
mod materials;

use bevy::prelude::*;

use super::{
    block_display::BLOCK_DISPLAY_FACES,
    block_model_material::BlockModelMaterial,
};
use crate::voxel::block_face::BlockFace;

pub(crate) use geometry::BlockModelMeshes;
pub(crate) use materials::{
    BlockModelMaterials, apply_block_display_shading, block_face_material_data,
    set_block_model_tint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockModelMode {
    Display,
    World,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct BlockModel {
    block_id: Option<&'static str>,
    mode: BlockModelMode,
    opacity: f32,
}

impl BlockModel {
    pub(crate) fn display(block_id: &'static str) -> Self {
        Self {
            block_id: Some(block_id),
            mode: BlockModelMode::Display,
            opacity: 1.0,
        }
    }

    pub(crate) fn empty_display() -> Self {
        Self {
            block_id: None,
            mode: BlockModelMode::Display,
            opacity: 1.0,
        }
    }

    pub(crate) fn world(block_id: &'static str, opacity: f32) -> Self {
        Self {
            block_id: Some(block_id),
            mode: BlockModelMode::World,
            opacity: opacity.clamp(0.0, 1.0),
        }
    }

    pub(crate) fn block_id(&self) -> Option<&'static str> {
        self.block_id
    }

    pub(crate) fn set_block_id(&mut self, block_id: Option<&'static str>) -> bool {
        if self.block_id == block_id {
            return false;
        }

        self.block_id = block_id;
        true
    }

    pub(crate) fn opacity(&self) -> f32 {
        self.opacity
    }

    pub(crate) fn faces(&self) -> &'static [BlockFace] {
        match self.mode {
            BlockModelMode::Display => &BLOCK_DISPLAY_FACES,
            BlockModelMode::World => &WORLD_FACES,
        }
    }
}

const WORLD_FACES: [BlockFace; 6] = BlockFace::ALL;

pub(crate) fn setup_block_model_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
) {
    commands.insert_resource(BlockModelMeshes::new(&mut meshes));
    commands.insert_resource(BlockModelMaterials::new(&mut materials));
}
