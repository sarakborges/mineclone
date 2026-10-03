use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::TypePath,
    render::{render_resource::AsBindGroup, storage::ShaderBuffer},
    shader::ShaderRef,
};

const TERRAIN_SHADER_PATH: &str = "shaders/terrain_material.wgsl";
const TERRAIN_VERTEX_SHADER_PATH: &str = "shaders/terrain_vertex.wgsl";
const TERRAIN_PREPASS_SHADER_PATH: &str = "shaders/terrain_prepass.wgsl";
const TERRAIN_PREPASS_VERTEX_SHADER_PATH: &str = "shaders/terrain_prepass_vertex.wgsl";

pub(crate) type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainMaterialExtension>;

#[derive(Resource, Clone)]
pub(crate) struct TerrainLightingBuffer {
    handle: Handle<ShaderBuffer>,
    sky_light_factor: f32,
    dynamic_light_enabled: f32,
    wind_x: f32,
    wind_z: f32,
}

impl TerrainLightingBuffer {
    pub(crate) fn new(buffers: &mut Assets<ShaderBuffer>) -> Self {
        let sky_light_factor = 1.0;
        let dynamic_light_enabled = 0.0;
        let wind_x = 0.0;
        let wind_z = 0.0;
        Self {
            handle: buffers.add(ShaderBuffer::from(vec![[
                sky_light_factor,
                dynamic_light_enabled,
                wind_x,
                wind_z,
            ]])),
            sky_light_factor,
            dynamic_light_enabled,
            wind_x,
            wind_z,
        }
    }

    pub(crate) fn handle(&self) -> Handle<ShaderBuffer> {
        self.handle.clone()
    }

    pub(crate) fn set_sky_light_factor(
        &mut self,
        buffers: &mut Assets<ShaderBuffer>,
        sky_light_factor: f32,
    ) {
        self.sky_light_factor = sky_light_factor;
        self.write(buffers);
    }

    pub(crate) fn set_dynamic_light_enabled(
        &mut self,
        buffers: &mut Assets<ShaderBuffer>,
        enabled: bool,
    ) {
        let next = enabled as u8 as f32;
        if self.dynamic_light_enabled == next {
            return;
        }
        self.dynamic_light_enabled = next;
        self.write(buffers);
    }

    pub(crate) fn set_wind_velocity(&mut self, buffers: &mut Assets<ShaderBuffer>, velocity: Vec3) {
        if self.wind_x == velocity.x && self.wind_z == velocity.z {
            return;
        }
        self.wind_x = velocity.x;
        self.wind_z = velocity.z;
        self.write(buffers);
    }

    fn write(&self, buffers: &mut Assets<ShaderBuffer>) {
        buffers
            .get_mut(&self.handle)
            .expect("terrain lighting buffer must remain loaded while a world is active")
            .set_data(vec![[
                self.sky_light_factor,
                self.dynamic_light_enabled,
                self.wind_x,
                self.wind_z,
            ]]);
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub(crate) struct TerrainMaterialExtension {
    #[storage(100, read_only)]
    pub lighting: Handle<ShaderBuffer>,
    #[uniform(101)]
    pub fluid_animation_factor: f32,
    #[uniform(101)]
    pub base_tint_enabled: f32,
    #[uniform(101)]
    pub overlay_enabled: f32,
    #[uniform(101)]
    pub overlay_tint_enabled: f32,
    #[uniform(101)]
    pub texture_array_enabled: f32,
    #[texture(102)]
    #[sampler(103)]
    pub overlay_texture: Option<Handle<Image>>,
    #[texture(104, dimension = "2d_array")]
    #[sampler(105)]
    pub terrain_texture_array: Handle<Image>,
}

impl MaterialExtension for TerrainMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        TERRAIN_VERTEX_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        TERRAIN_PREPASS_VERTEX_SHADER_PATH.into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        TERRAIN_PREPASS_SHADER_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        TERRAIN_SHADER_PATH.into()
    }
}
