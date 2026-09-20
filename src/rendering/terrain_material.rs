use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::TypePath,
    render::{render_resource::AsBindGroup, storage::ShaderBuffer},
    shader::ShaderRef,
};

const TERRAIN_SHADER_PATH: &str = "shaders/terrain_material.wgsl";

pub(crate) type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainMaterialExtension>;

#[derive(Resource, Clone)]
pub(crate) struct TerrainLightingBuffer(Handle<ShaderBuffer>);

impl TerrainLightingBuffer {
    pub(crate) fn new(buffers: &mut Assets<ShaderBuffer>) -> Self {
        Self(buffers.add(ShaderBuffer::from(vec![[1.0_f32, 0.0, 0.0, 0.0]])))
    }

    pub(crate) fn handle(&self) -> Handle<ShaderBuffer> {
        self.0.clone()
    }

    pub(crate) fn set_sky_light_factor(
        &self,
        buffers: &mut Assets<ShaderBuffer>,
        sky_light_factor: f32,
    ) {
        buffers
            .get_mut(&self.0)
            .expect("terrain lighting buffer must remain loaded while a world is active")
            .set_data(vec![[sky_light_factor, 0.0, 0.0, 0.0]]);
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub(crate) struct TerrainMaterialExtension {
    #[storage(100, read_only)]
    pub lighting: Handle<ShaderBuffer>,
    #[uniform(101)]
    pub fluid_animation_factor: f32,
    #[uniform(101)]
    pub tint_enabled: f32,
}

impl MaterialExtension for TerrainMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }
}
