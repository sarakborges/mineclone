mod geometry;
mod materials;

use bevy::prelude::*;

use super::{block_display::BLOCK_DISPLAY_FACES, block_model_material::BlockModelMaterial};
use crate::{
    content::block::BlockRegistry,
    voxel::{block_face::BlockFace, log_variant::is_hollow_log_id},
};

pub(crate) use geometry::BlockModelMeshes;
pub(crate) use materials::{
    BlockModelMaterials, apply_block_display_shading, block_face_material_data,
    maximum_block_model_layers, set_block_model_tint,
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

    pub(crate) fn empty_world(opacity: f32) -> Self {
        Self {
            block_id: None,
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

/// Block-model faces are shared by held items and placement previews. Swap only
/// the mesh handle when a model changes between a full cube and a 1/8 layer;
/// materials and transforms keep their existing ownership and update paths.
pub(crate) fn sync_block_model_mesh_geometry(
    blocks: Res<BlockRegistry>,
    block_meshes: Res<BlockModelMeshes>,
    models: Query<&BlockModel>,
    parents: Query<&ChildOf>,
    mut mesh_faces: Query<(Entity, &mut Mesh3d)>,
) {
    for (entity, mut mesh) in &mut mesh_faces {
        let Ok(parent) = parents.get(entity) else {
            continue;
        };
        let Ok(model) = models.get(parent.parent()) else {
            continue;
        };
        let Some(block_id) = model.block_id() else {
            continue;
        };
        let Some(block) = blocks.get(block_id) else {
            continue;
        };
        let Some(face) = block_meshes.face_for_mesh(&mesh.0) else {
            continue;
        };

        let next = match model.mode {
            BlockModelMode::Display => block_meshes.display_face_for_block(face, block),
            BlockModelMode::World if is_hollow_log_id(block_id) => {
                block_meshes.hollow_world_face(face)
            }
            BlockModelMode::World => block_meshes.world_face_for_block(face, block),
        };
        if mesh.0 != next {
            mesh.0 = next;
        }
    }
}
