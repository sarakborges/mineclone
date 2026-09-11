use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry},
        fluid::{FluidId, FluidRegistry},
    },
    rendering::{
        block_texture::block_face_texture,
        terrain_material::{TerrainMaterial, TerrainMaterialExtension},
    },
    voxel::block_face::{BlockFace, BlockFaces},
};

#[derive(Resource, Clone)]
pub struct TerrainMaterials {
    blocks: HashMap<String, BlockFaces<Handle<TerrainMaterial>>>,
    _texture_preloads: Vec<Handle<Image>>,
}

impl TerrainMaterials {
    pub fn from_registry(
        blocks: &BlockRegistry,
        asset_server: &AssetServer,
        materials: &mut Assets<TerrainMaterial>,
        roughness: f32,
        metallic: f32,
    ) -> Self {
        let mut seen_textures = HashSet::<String>::new();
        let texture_preloads = blocks
            .iter()
            .flat_map(|definition| {
                BlockFace::ALL
                    .into_iter()
                    .filter_map(move |face| block_face_texture(face, definition))
            })
            .filter_map(|texture| {
                let texture = texture.to_owned();
                seen_textures.insert(texture.clone()).then_some(texture)
            })
            .map(|texture| asset_server.load(texture))
            .collect();

        let blocks = blocks
            .iter()
            .map(|definition| {
                let block_materials = BlockFaces::from_fn(|face| {
                    create_material(
                        definition,
                        face,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    )
                });

                (definition.id.clone(), block_materials)
            })
            .collect();

        Self {
            blocks,
            _texture_preloads: texture_preloads,
        }
    }

    pub(super) fn for_face(&self, block_id: &str, face: BlockFace) -> &Handle<TerrainMaterial> {
        self.blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("missing terrain materials for block: {block_id}"))
            .get(face)
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
            alpha_mode: definition.alpha_mode(1.0),
            fog_enabled: false,
            unlit: false,
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
                        base_color: Color::srgba(1.0, 1.0, 1.0, definition.opacity),
                        perceptual_roughness: definition.roughness,
                        metallic: definition.metallic,
                        alpha_mode: AlphaMode::Blend,
                        double_sided: true,
                        cull_mode: None,
                        fog_enabled: false,
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
