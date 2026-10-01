use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

use crate::content::{block::BlockDefinition, block_orientation::BlockOrientation};

const BLOCK_ICON_SHADER_PATH: &str = "shaders/block_icon_material.wgsl";
const BLOCK_ICON_DIRECTORY: &str = "block_icons";

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub(crate) struct BlockIconMaterial {
    #[texture(0)]
    #[sampler(1)]
    icon_texture: Handle<Image>,
}

impl UiMaterial for BlockIconMaterial {
    fn fragment_shader() -> ShaderRef {
        BLOCK_ICON_SHADER_PATH.into()
    }
}

impl BlockIconMaterial {
    pub(crate) fn empty() -> Self {
        Self {
            icon_texture: Handle::default(),
        }
    }

    pub(crate) fn from_block(
        block: &BlockDefinition,
        asset_server: &AssetServer,
        _tint: Color,
    ) -> Self {
        let mut material = Self::empty();
        material.set_block(block, asset_server);
        material
    }

    pub(crate) fn set_block(&mut self, block: &BlockDefinition, asset_server: &AssetServer) {
        self.icon_texture = asset_server.load(block_icon_asset_path(&block.id));
    }

    pub(crate) fn set_block_orientation(
        &mut self,
        block: &BlockDefinition,
        _orientation: BlockOrientation,
        asset_server: &AssetServer,
    ) {
        self.set_block(block, asset_server);
    }

    // Block icons are authored PNGs. Biome/dye tinting belongs in the authored
    // icon instead of being recomputed by the HUD renderer.
    pub(crate) fn has_tint(&self, _tint: Color) -> bool {
        true
    }

    pub(crate) fn set_tint(&mut self, _tint: Color) {}
}

fn block_icon_asset_path(block_id: &str) -> String {
    format!("{BLOCK_ICON_DIRECTORY}/{block_id}.png")
}
