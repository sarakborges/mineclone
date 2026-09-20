use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry, BlockTextureLayer},
        fluid::{FluidId, FluidRegistry},
    },
    rendering::{
        block_texture::{block_face_texture_layers, load_block_texture_layer},
        terrain_material::{
            TerrainLightingBuffer, TerrainMaterial, TerrainMaterialExtension,
        },
    },
    voxel::block_face::{BlockFace, BlockFaces},
};

const TEXTURE_LAYER_DEPTH_BIAS: f32 = 2.0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TerrainAlphaKey {
    Opaque,
    Mask(u32),
    Blend,
}

impl TerrainAlphaKey {
    fn for_layer(definition: &BlockDefinition, layer_index: usize) -> Self {
        if layer_index > 0 || definition.alpha_blend {
            Self::Blend
        } else if let Some(cutoff) = definition.alpha_cutoff {
            Self::Mask(cutoff.to_bits())
        } else {
            Self::Opaque
        }
    }

    fn alpha_mode(self) -> AlphaMode {
        match self {
            Self::Opaque => AlphaMode::Opaque,
            Self::Mask(cutoff) => AlphaMode::Mask(f32::from_bits(cutoff)),
            Self::Blend => AlphaMode::Blend,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TerrainMaterialKey {
    texture: Option<String>,
    tint_enabled: bool,
    alpha: TerrainAlphaKey,
    layer_index: usize,
}


struct TerrainMaterialBuilder<'a> {
    asset_server: &'a AssetServer,
    materials: &'a mut Assets<TerrainMaterial>,
    cache: HashMap<TerrainMaterialKey, Handle<TerrainMaterial>>,
    lighting: &'a TerrainLightingBuffer,
    roughness: f32,
    metallic: f32,
}

impl<'a> TerrainMaterialBuilder<'a> {
    fn new(
        asset_server: &'a AssetServer,
        materials: &'a mut Assets<TerrainMaterial>,
        lighting: &'a TerrainLightingBuffer,
        roughness: f32,
        metallic: f32,
    ) -> Self {
        Self {
            asset_server,
            materials,
            cache: HashMap::new(),
            lighting,
            roughness,
            metallic,
        }
    }

    fn layers_for(
        &mut self,
        definition: &BlockDefinition,
        face: BlockFace,
    ) -> Vec<Handle<TerrainMaterial>> {
        let layers = block_face_texture_layers(face, definition);
        if layers.is_empty() {
            return vec![self.material_for(definition, None, 0)];
        }

        layers
            .iter()
            .enumerate()
            .map(|(layer_index, layer)| {
                self.material_for(definition, Some(layer), layer_index)
            })
            .collect()
    }

    fn material_for(
        &mut self,
        definition: &BlockDefinition,
        layer: Option<&BlockTextureLayer>,
        layer_index: usize,
    ) -> Handle<TerrainMaterial> {
        let alpha = TerrainAlphaKey::for_layer(definition, layer_index);
        let key = TerrainMaterialKey {
            texture: layer.map(|layer| layer.texture.clone()),
            tint_enabled: layer.is_some_and(|layer| layer.dyable),
            alpha,
            layer_index,
        };
        if let Some(existing) = self.cache.get(&key) {
            return existing.clone();
        }

        let base_color_texture =
            layer.map(|layer| load_block_texture_layer(self.asset_server, layer));
        let tint_enabled = key.tint_enabled as u8 as f32;
        let material = self.materials.add(TerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture,
                perceptual_roughness: self.roughness,
                metallic: self.metallic,
                alpha_mode: alpha.alpha_mode(),
                depth_bias: layer_index as f32 * TEXTURE_LAYER_DEPTH_BIAS,
                fog_enabled: true,
                unlit: true,
                ..default()
            },
            extension: TerrainMaterialExtension {
                lighting: self.lighting.handle(),
                fluid_animation_factor: 0.0,
                tint_enabled,
            },
        });
        self.cache.insert(key, material.clone());
        material
    }
}

#[derive(Resource, Clone)]
pub struct TerrainMaterials {
    blocks: HashMap<String, BlockFaces<Vec<Handle<TerrainMaterial>>>>,
    _texture_preloads: Vec<Handle<Image>>,
}

impl TerrainMaterials {
    pub fn from_registry(
        blocks: &BlockRegistry,
        asset_server: &AssetServer,
        materials: &mut Assets<TerrainMaterial>,
        lighting: &TerrainLightingBuffer,
        roughness: f32,
        metallic: f32,
    ) -> Self {
        let mut seen_textures = HashSet::<String>::new();
        let texture_preloads = blocks
            .iter()
            .flat_map(|definition| {
                BlockFace::ALL.into_iter().flat_map(move |face| {
                    block_face_texture_layers(face, definition)
                        .iter()
                        .map(|layer| layer.texture.as_str())
                })
            })
            .filter_map(|texture| {
                let texture = texture.to_owned();
                seen_textures.insert(texture.clone()).then_some(texture)
            })
            .map(|texture| asset_server.load(texture))
            .collect();

        let mut builder =
            TerrainMaterialBuilder::new(asset_server, materials, lighting, roughness, metallic);
        let blocks = blocks
            .iter()
            .map(|definition| {
                let block_materials =
                    BlockFaces::from_fn(|face| builder.layers_for(definition, face));
                (definition.id.clone(), block_materials)
            })
            .collect();

        Self {
            blocks,
            _texture_preloads: texture_preloads,
        }
    }

    pub(super) fn for_face(
        &self,
        block_id: &str,
        face: BlockFace,
    ) -> &[Handle<TerrainMaterial>] {
        self.blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("missing terrain materials for block: {block_id}"))
            .get(face)
            .as_slice()
    }
}

#[derive(Resource, Clone)]
pub struct FluidMaterials {
    materials: HashMap<FluidId, Handle<TerrainMaterial>>,
}

impl FluidMaterials {
    pub fn from_registry(
        fluids: &FluidRegistry,
        materials: &mut Assets<TerrainMaterial>,
        lighting: &TerrainLightingBuffer,
    ) -> Self {
        let materials = fluids
            .iter()
            .map(|(fluid_id, definition)| {
                let material = materials.add(TerrainMaterial {
                    base: StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 1.0, definition.opacity),
                        perceptual_roughness: definition.roughness,
                        metallic: definition.metallic,
                        alpha_mode: if definition.opacity >= 1.0 {
                            AlphaMode::Opaque
                        } else {
                            AlphaMode::Blend
                        },
                        double_sided: true,
                        cull_mode: None,
                        fog_enabled: true,
                        unlit: true,
                        ..default()
                    },
                    extension: TerrainMaterialExtension {
                        lighting: lighting.handle(),
                        fluid_animation_factor: 1.0,
                        tint_enabled: 1.0,
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
