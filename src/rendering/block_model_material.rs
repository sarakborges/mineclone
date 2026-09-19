use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

use crate::rendering::color::color_to_linear_vec4;

const BLOCK_MODEL_SHADER_PATH: &str = "shaders/block_model_material.wgsl";

pub(crate) type BlockModelMaterial =
    ExtendedMaterial<StandardMaterial, BlockModelMaterialExtension>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub(crate) struct BlockModelMaterialExtension {
    #[uniform(100)]
    tint: Vec4,
    #[uniform(100)]
    tint_enabled: f32,
    #[uniform(100)]
    opacity: f32,
}

impl Default for BlockModelMaterialExtension {
    fn default() -> Self {
        Self {
            tint: Vec4::ONE,
            tint_enabled: 0.0,
            opacity: 1.0,
        }
    }
}

impl BlockModelMaterialExtension {
    pub(crate) fn set_dyable(&mut self, dyable: bool) {
        self.tint_enabled = dyable as u8 as f32;
    }

    pub(crate) fn set_tint(&mut self, color: Color) {
        self.tint = color_to_linear_vec4(color);
    }

    pub(crate) fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }
}

impl MaterialExtension for BlockModelMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        BLOCK_MODEL_SHADER_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        BLOCK_MODEL_SHADER_PATH.into()
    }
}
