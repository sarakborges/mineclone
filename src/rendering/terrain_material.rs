use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

const TERRAIN_SHADER_PATH: &str = "shaders/terrain_material.wgsl";

pub(crate) type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainMaterialExtension>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub(crate) struct TerrainMaterialExtension {}

impl MaterialExtension for TerrainMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }
}
