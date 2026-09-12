use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

const TERRAIN_SHADER_PATH: &str = "shaders/terrain_material.wgsl";

pub(crate) type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainMaterialExtension>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub(crate) struct TerrainMaterialExtension {
    #[uniform(100)]
    pub sky_light_factor: f32,
    #[uniform(100)]
    pub fluid_animation_factor: f32,
    #[uniform(100)]
    pub padding: Vec2,
}

impl Default for TerrainMaterialExtension {
    fn default() -> Self {
        Self {
            sky_light_factor: 1.0,
            fluid_animation_factor: 0.0,
            padding: Vec2::ZERO,
        }
    }
}

impl MaterialExtension for TerrainMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }
}
