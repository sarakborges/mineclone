use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

use crate::content::block::BlockDefinition;

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
}

impl UiMaterial for BlockIconMaterial {
    fn fragment_shader() -> ShaderRef {
        BLOCK_ICON_SHADER_PATH.into()
    }
}

impl BlockIconMaterial {
    pub(crate) fn empty() -> Self {
        Self {
            top_texture: Handle::default(),
            front_texture: Handle::default(),
            right_texture: Handle::default(),
            tint: Vec4::ONE,
        }
    }

    pub(crate) fn from_block(
        block: &BlockDefinition,
        asset_server: &AssetServer,
        tint: Color,
    ) -> Self {
        let fallback = first_texture(block);

        Self {
            top_texture: load_texture(asset_server, &block.textures.top, fallback),
            front_texture: load_texture(asset_server, &block.textures.front, fallback),
            right_texture: load_texture(asset_server, &block.textures.right, fallback),
            tint: tint_vec4(tint),
        }
    }

    pub(crate) fn set_block(&mut self, block: &BlockDefinition, asset_server: &AssetServer) {
        let fallback = first_texture(block);
        self.top_texture = load_texture(asset_server, &block.textures.top, fallback);
        self.front_texture = load_texture(asset_server, &block.textures.front, fallback);
        self.right_texture = load_texture(asset_server, &block.textures.right, fallback);
    }

    pub(crate) fn set_tint(&mut self, tint: Color) {
        self.tint = tint_vec4(tint);
    }
}

fn first_texture(block: &BlockDefinition) -> &str {
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
    .unwrap_or("")
}

fn load_texture(asset_server: &AssetServer, texture: &str, fallback: &str) -> Handle<Image> {
    let path = if texture.is_empty() { fallback } else { texture };

    if path.is_empty() {
        Handle::default()
    } else {
        asset_server.load(path.to_owned())
    }
}

fn tint_vec4(tint: Color) -> Vec4 {
    let tint = tint.to_linear();
    Vec4::new(tint.red, tint.green, tint.blue, tint.alpha)
}
