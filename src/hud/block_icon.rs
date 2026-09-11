use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

use crate::{
    content::block::BlockDefinition,
    rendering::{
        block_model::{block_display_face_basis, block_display_face_shade},
        block_texture::block_face_texture,
        color::color_to_linear_vec4,
    },
    voxel::block_face::BlockFace,
};

const BLOCK_ICON_SHADER_PATH: &str = "shaders/block_icon_material.wgsl";

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub(crate) struct BlockIconMaterial {
    #[texture(0)]
    #[sampler(1)]
    top_texture: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    front_texture: Handle<Image>,
    #[texture(4)]
    #[sampler(5)]
    right_texture: Handle<Image>,
    #[uniform(6)]
    tint: Vec4,
    #[uniform(7)]
    face_shades: Vec4,
    #[uniform(8)]
    top_origin_axis_u: Vec4,
    #[uniform(9)]
    top_axis_v: Vec4,
    #[uniform(10)]
    front_origin_axis_u: Vec4,
    #[uniform(11)]
    front_axis_v: Vec4,
    #[uniform(12)]
    right_origin_axis_u: Vec4,
    #[uniform(13)]
    right_axis_v: Vec4,
}

impl UiMaterial for BlockIconMaterial {
    fn fragment_shader() -> ShaderRef {
        BLOCK_ICON_SHADER_PATH.into()
    }
}

impl BlockIconMaterial {
    pub(crate) fn empty() -> Self {
        let (top_origin_axis_u, top_axis_v) = block_display_face_basis(BlockFace::Top);
        let (front_origin_axis_u, front_axis_v) = block_display_face_basis(BlockFace::Front);
        let (right_origin_axis_u, right_axis_v) = block_display_face_basis(BlockFace::Right);

        Self {
            top_texture: Handle::default(),
            front_texture: Handle::default(),
            right_texture: Handle::default(),
            tint: Vec4::ONE,
            face_shades: block_face_shades(),
            top_origin_axis_u,
            top_axis_v,
            front_origin_axis_u,
            front_axis_v,
            right_origin_axis_u,
            right_axis_v,
        }
    }

    pub(crate) fn from_block(
        block: &BlockDefinition,
        asset_server: &AssetServer,
        tint: Color,
    ) -> Self {
        let mut material = Self::empty();
        material.set_block(block, asset_server);
        material.set_tint(tint);
        material
    }

    pub(crate) fn set_block(&mut self, block: &BlockDefinition, asset_server: &AssetServer) {
        self.top_texture = load_face_texture(asset_server, block, BlockFace::Top);
        self.front_texture = load_face_texture(asset_server, block, BlockFace::Front);
        self.right_texture = load_face_texture(asset_server, block, BlockFace::Right);
    }

    pub(crate) fn set_tint(&mut self, tint: Color) {
        self.tint = color_to_linear_vec4(tint);
    }
}

fn block_face_shades() -> Vec4 {
    Vec4::new(
        block_display_face_shade(BlockFace::Top),
        block_display_face_shade(BlockFace::Front),
        block_display_face_shade(BlockFace::Right),
        1.0,
    )
}

fn load_face_texture(
    asset_server: &AssetServer,
    block: &BlockDefinition,
    face: BlockFace,
) -> Handle<Image> {
    block_face_texture(face, block)
        .map(|texture| asset_server.load(texture.to_owned()))
        .unwrap_or_default()
}
