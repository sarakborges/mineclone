use bevy::prelude::*;

use crate::{
    content::block::{BlockDefinition, BlockRegistry},
    rendering::{
        block_display::block_display_face_shade,
        block_model_material::{BlockModelMaterial, BlockModelMaterialExtension},
        block_texture::{block_face_texture_layers, load_block_texture_layer},
    },
    voxel::block_face::{BlockFace, BlockFaces},
};

const TEXTURE_LAYER_DEPTH_BIAS: f32 = 2.0;

#[derive(Resource)]
pub(crate) struct BlockModelMaterials {
    held: BlockFaces<Vec<Handle<BlockModelMaterial>>>,
    preview: BlockFaces<Vec<Handle<BlockModelMaterial>>>,
}

impl BlockModelMaterials {
    pub(super) fn new(_materials: &mut Assets<BlockModelMaterial>) -> Self {
        Self {
            held: BlockFaces::from_fn(|_| Vec::new()),
            preview: BlockFaces::from_fn(|_| Vec::new()),
        }
    }

    pub(crate) fn held_for_face(
        &mut self,
        face: BlockFace,
        layer_count: usize,
        materials: &mut Assets<BlockModelMaterial>,
    ) -> Vec<Handle<BlockModelMaterial>> {
        ensure_material_slots(self.held.get_mut(face), layer_count, 1.0, materials);
        self.held.get(face)[..layer_count].to_vec()
    }

    pub(crate) fn preview_for_face(
        &mut self,
        face: BlockFace,
        layer_count: usize,
        materials: &mut Assets<BlockModelMaterial>,
    ) -> Vec<Handle<BlockModelMaterial>> {
        ensure_material_slots(self.preview.get_mut(face), layer_count, 0.68, materials);
        self.preview.get(face)[..layer_count].to_vec()
    }
}

pub(crate) fn maximum_block_model_layers(blocks: &BlockRegistry, face: BlockFace) -> usize {
    blocks
        .iter()
        .map(|block| block_face_texture_layers(face, block).len().max(1))
        .max()
        .unwrap_or(1)
}

pub(crate) fn block_face_material_data(
    face: BlockFace,
    layer_index: usize,
    block: &BlockDefinition,
    asset_server: &AssetServer,
    opacity: f32,
) -> Option<BlockModelMaterial> {
    let opacity = opacity.clamp(0.0, 1.0);
    let layers = block_face_texture_layers(face, block);
    let layer = if layers.is_empty() {
        if layer_index != 0 {
            return None;
        }
        None
    } else {
        Some(layers.get(layer_index)?)
    };
    let alpha_mode = if layer_index == 0 {
        block.alpha_mode(opacity)
    } else {
        AlphaMode::Blend
    };
    let mut extension = BlockModelMaterialExtension::default();
    extension.set_dyable(layer.is_some_and(|layer| layer.dyable));

    Some(BlockModelMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
            base_color_texture: layer.map(|layer| load_block_texture_layer(asset_server, layer)),
            perceptual_roughness: 1.0,
            alpha_mode,
            depth_bias: layer_index as f32 * TEXTURE_LAYER_DEPTH_BIAS,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension,
    })
}

pub(crate) fn set_block_model_tint(material: &mut BlockModelMaterial, tint: Color) {
    material.extension.set_tint(tint);
}

pub(crate) fn apply_block_display_shading(
    material: &mut BlockModelMaterial,
    face: BlockFace,
    opacity: f32,
) {
    let shade = block_display_face_shade(face);
    material.base.base_color = Color::srgba(shade, shade, shade, opacity.clamp(0.0, 1.0));
}

fn ensure_material_slots(
    slots: &mut Vec<Handle<BlockModelMaterial>>,
    layer_count: usize,
    opacity: f32,
    materials: &mut Assets<BlockModelMaterial>,
) {
    while slots.len() < layer_count {
        let layer_index = slots.len();
        slots.push(materials.add(block_model_placeholder_material(opacity, layer_index)));
    }
}

fn block_model_placeholder_material(opacity: f32, layer_index: usize) -> BlockModelMaterial {
    let opacity = opacity.clamp(0.0, 1.0);

    BlockModelMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
            perceptual_roughness: 1.0,
            alpha_mode: if layer_index > 0 || opacity < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            depth_bias: layer_index as f32 * TEXTURE_LAYER_DEPTH_BIAS,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: BlockModelMaterialExtension::default(),
    }
}
