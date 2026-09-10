use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

const BLOCK_MODEL_SHADER_PATH: &str = "shaders/block_model_material.wgsl";

pub(crate) type BlockModelMaterial =
    ExtendedMaterial<StandardMaterial, BlockModelMaterialExtension>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub(crate) struct BlockModelMaterialExtension {
    #[uniform(100)]
    tint: Vec4,
}

impl Default for BlockModelMaterialExtension {
    fn default() -> Self {
        Self { tint: Vec4::ONE }
    }
}

impl BlockModelMaterialExtension {
    pub(crate) fn set_tint(&mut self, color: Color) {
        let color = color.to_linear();
        self.tint = Vec4::new(color.red, color.green, color.blue, color.alpha);
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
