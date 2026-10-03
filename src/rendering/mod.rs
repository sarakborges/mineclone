mod ambient_particles;
mod asset_upload;
pub(crate) mod biome_visuals;
pub(crate) mod block_display;
pub(crate) mod block_model;
pub(crate) mod block_model_material;
pub(crate) mod block_texture;
pub(crate) mod block_tint;
pub(crate) mod block_visual_content;
pub(crate) mod camera_stack;
mod celestial;
mod celestial_path;
pub(crate) mod color;
mod dynamic_lights;
mod environment;
pub(crate) mod extruded_sprite;
mod fog;
mod gameplay_asset_preload;
mod lighting;
mod mesh_allocator_diagnostics;
pub(crate) mod object_primitives;
mod sky;
mod sky_layers;
mod sun_lighting;
pub(crate) mod terrain_material;
pub(crate) mod wind;

use ambient_particles::AmbientParticlesPlugin;
use asset_upload::AssetUploadPlugin;
use bevy::{prelude::*, render::storage::ShaderBuffer};
use block_model::{setup_block_model_assets, sync_block_model_mesh_geometry};
use block_model_material::BlockModelMaterial;
use celestial::CelestialPlugin;
use dynamic_lights::DynamicLightsPlugin;
use environment::EnvironmentPlugin;
use extruded_sprite::ExtrudedSpritePlugin;
use fog::FogPlugin;
use lighting::LightingPlugin;
use mesh_allocator_diagnostics::MeshAllocatorDiagnosticsPlugin;
use object_primitives::ObjectPrimitivesPlugin;
use sky::SkyPlugin;
use sky_layers::SkyLayersPlugin;
use sun_lighting::SunLightingPlugin;
use terrain_material::{TerrainLightingBuffer, TerrainMaterial};
use wind::Wind;

pub(crate) use gameplay_asset_preload::GameplayAssetPreloads;

pub(crate) struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameplayAssetPreloads>()
            .init_resource::<Wind>()
            .add_plugins((
                MaterialPlugin::<TerrainMaterial>::default(),
                MaterialPlugin::<BlockModelMaterial>::default(),
            ))
            .add_systems(Startup, setup_block_model_assets)
            .add_systems(Update, sync_terrain_wind)
            .add_systems(PostUpdate, sync_block_model_mesh_geometry)
            .add_plugins((
                AssetUploadPlugin,
                ExtrudedSpritePlugin,
                ObjectPrimitivesPlugin,
                MeshAllocatorDiagnosticsPlugin,
                EnvironmentPlugin,
                LightingPlugin,
                SunLightingPlugin,
                DynamicLightsPlugin,
                FogPlugin,
                SkyPlugin,
                SkyLayersPlugin,
                CelestialPlugin,
                AmbientParticlesPlugin,
            ));
    }
}

fn sync_terrain_wind(
    wind: Res<Wind>,
    terrain_lighting: Option<ResMut<TerrainLightingBuffer>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(mut terrain_lighting) = terrain_lighting else {
        return;
    };
    terrain_lighting.set_wind_velocity(&mut buffers, wind.velocity(1.0));
}
