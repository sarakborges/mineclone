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
pub(crate) mod extruded_sprite;
mod sun_lighting;
mod dynamic_lights;
mod environment;
mod fog;
mod gameplay_asset_preload;
mod lighting;
mod mesh_allocator_diagnostics;
mod sky;
mod sky_layers;
pub(crate) mod terrain_material;

use asset_upload::AssetUploadPlugin;
use bevy::prelude::*;
use block_model::setup_block_model_assets;
use block_model_material::BlockModelMaterial;
use celestial::CelestialPlugin;
use sun_lighting::SunLightingPlugin;
use dynamic_lights::DynamicLightsPlugin;
use environment::EnvironmentPlugin;
use extruded_sprite::ExtrudedSpritePlugin;
use fog::FogPlugin;
use lighting::LightingPlugin;
use mesh_allocator_diagnostics::MeshAllocatorDiagnosticsPlugin;
use sky::SkyPlugin;
use sky_layers::SkyLayersPlugin;
use terrain_material::TerrainMaterial;

pub(crate) use gameplay_asset_preload::GameplayAssetPreloads;

pub(crate) struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameplayAssetPreloads>()
            .add_plugins((
            MaterialPlugin::<TerrainMaterial>::default(),
            MaterialPlugin::<BlockModelMaterial>::default(),
        ))
        .add_systems(Startup, setup_block_model_assets)
        .add_plugins((
            AssetUploadPlugin,
            ExtrudedSpritePlugin,
            MeshAllocatorDiagnosticsPlugin,
            EnvironmentPlugin,
            LightingPlugin,
            SunLightingPlugin,
            DynamicLightsPlugin,
            FogPlugin,
            SkyPlugin,
            SkyLayersPlugin,
            CelestialPlugin,
        ));
    }
}
