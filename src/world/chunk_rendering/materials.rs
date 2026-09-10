use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry},
        fluid::{FluidId, FluidRegistry},
    },
    rendering::{
        block_model::block_face_texture,
        terrain_material::{TerrainMaterial, TerrainMaterialExtension},
    },
    voxel::mesh::BlockFace,
};

#[derive(Clone)]
struct BlockTerrainMaterials {
    top: Handle<TerrainMaterial>,
    bottom: Handle<TerrainMaterial>,
    left: Handle<TerrainMaterial>,
    right: Handle<TerrainMaterial>,
    front: Handle<TerrainMaterial>,
    back: Handle<TerrainMaterial>,
}

impl BlockTerrainMaterials {
    fn for_face(&self, face: BlockFace) -> &Handle<TerrainMaterial> {
        match face {
            BlockFace::Right => &self.right,
            BlockFace::Left => &self.left,
            BlockFace::Top => &self.top,
            BlockFace::Bottom => &self.bottom,
            BlockFace::Front => &self.front,
            BlockFace::Back => &self.back,
        }
    }
}

#[derive(Resource, Clone)]
pub struct TerrainMaterials {
    blocks: HashMap<String, BlockTerrainMaterials>,
}

impl TerrainMaterials {
    pub fn from_registry(
        blocks: &BlockRegistry,
        asset_server: &AssetServer,
        materials: &mut Assets<TerrainMaterial>,
        roughness: f32,
        metallic: f32,
    ) -> Self {
        let blocks = blocks
            .iter()
            .map(|definition| {
                let block_materials = BlockTerrainMaterials {
                    top: create_material(
                        definition,
                        BlockFace::Top,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    bottom: create_material(
                        definition,
                        BlockFace::Bottom,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    left: create_material(
                        definition,
                        BlockFace::Left,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    right: create_material(
                        definition,
                        BlockFace::Right,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    front: create_material(
                        definition,
                        BlockFace::Front,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    back: create_material(
                        definition,
                        BlockFace::Back,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                };

                (definition.id.clone(), block_materials)
            })
            .collect();

        Self { blocks }
    }

    pub(super) fn for_face(&self, block_id: &str, face: BlockFace) -> &Handle<TerrainMaterial> {
        self.blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("missing terrain materials for block: {block_id}"))
            .for_face(face)
    }
}

fn create_material(
    definition: &BlockDefinition,
    face: BlockFace,
    asset_server: &AssetServer,
    materials: &mut Assets<TerrainMaterial>,
    roughness: f32,
    metallic: f32,
) -> Handle<TerrainMaterial> {
    materials.add(TerrainMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: block_face_texture(face, definition)
                .map(|texture| asset_server.load(texture.to_owned())),
            perceptual_roughness: roughness,
            metallic,
            unlit: true,
            ..default()
        },
        extension: TerrainMaterialExtension::default(),
    })
}

#[derive(Resource, Clone)]
pub struct FluidMaterials {
    materials: HashMap<FluidId, Handle<TerrainMaterial>>,
}

impl FluidMaterials {
    pub fn from_registry(fluids: &FluidRegistry, materials: &mut Assets<TerrainMaterial>) -> Self {
        let materials = fluids
            .iter()
            .map(|(fluid_id, definition)| {
                let material = materials.add(TerrainMaterial {
                    base: StandardMaterial {
                        base_color: Color::srgba(
                            definition.color.r,
                            definition.color.g,
                            definition.color.b,
                            definition.opacity,
                        ),
                        perceptual_roughness: definition.roughness,
                        metallic: definition.metallic,
                        alpha_mode: AlphaMode::Blend,
                        double_sided: true,
                        cull_mode: None,
                        unlit: true,
                        ..default()
                    },
                    extension: TerrainMaterialExtension {
                        fluid_animation_factor: 1.0,
                        ..default()
                    },
                });

                (fluid_id, material)
            })
            .collect();

        Self { materials }
    }

    pub(super) fn get(&self, fluid_id: FluidId) -> &Handle<TerrainMaterial> {
        self.materials
            .get(&fluid_id)
            .unwrap_or_else(|| panic!("missing material for fluid id {fluid_id}"))
    }
}
