use bevy::prelude::*;

use crate::{
    content::block::BlockDefinition,
    rendering::block_model_material::{BlockModelMaterial, BlockModelMaterialExtension},
    voxel::block_face::{BlockFace, BlockFaces},
};

use super::geometry::block_display_face_shade;

#[derive(Resource)]
pub(crate) struct BlockModelMaterials {
    held: BlockFaces<Handle<BlockModelMaterial>>,
    preview: BlockFaces<Handle<BlockModelMaterial>>,
}

impl BlockModelMaterials {
    pub(super) fn new(materials: &mut Assets<BlockModelMaterial>) -> Self {
        Self {
            held: BlockFaces::from_fn(|_| materials.add(block_model_placeholder_material(1.0))),
            preview: BlockFaces::from_fn(|_| {
                materials.add(block_model_placeholder_material(0.68))
            }),
        }
    }

    pub(crate) fn held_for_face(&self, face: BlockFace) -> Handle<BlockModelMaterial> {
        self.held.get(face).clone()
    }

    pub(crate) fn preview_for_face(&self, face: BlockFace) -> Handle<BlockModelMaterial> {
        self.preview.get(face).clone()
    }
}

pub(crate) fn block_face_texture(face: BlockFace, block: &BlockDefinition) -> Option<&str> {
    let texture = match face {
        BlockFace::Right => block.textures.right.as_str(),
        BlockFace::Left => block.textures.left.as_str(),
        BlockFace::Top => block.textures.top.as_str(),
        BlockFace::Bottom => block.textures.bottom.as_str(),
        BlockFace::Front => block.textures.front.as_str(),
        BlockFace::Back => block.textures.back.as_str(),
    };

    if !texture.is_empty() {
        Some(texture)
    } else {
        first_block_texture(block)
    }
}

fn first_block_texture(block: &BlockDefinition) -> Option<&str> {
    [
        block.textures.top.as_str(),
        block.textures.front.as_str(),
        block.textures.right.as_str(),
        block.textures.left.as_str(),
        block.textures.back.as_str(),
        block.textures.bottom.as_str(),
    ]
    .into_iter()
    .find(|texture| !texture.is_empty())
}

pub(crate) fn block_face_material_data(
    face: BlockFace,
    block: &BlockDefinition,
    asset_server: &AssetServer,
    opacity: f32,
) -> BlockModelMaterial {
    let opacity = opacity.clamp(0.0, 1.0);

    BlockModelMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
            base_color_texture: block_face_texture(face, block)
                .map(|texture| asset_server.load(texture.to_owned())),
            perceptual_roughness: 1.0,
            alpha_mode: block.alpha_mode(opacity),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: BlockModelMaterialExtension::default(),
    }
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

fn block_model_placeholder_material(opacity: f32) -> BlockModelMaterial {
    let opacity = opacity.clamp(0.0, 1.0);

    BlockModelMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
            perceptual_roughness: 1.0,
            alpha_mode: if opacity < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: BlockModelMaterialExtension::default(),
    }
}
