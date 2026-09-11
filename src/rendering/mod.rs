mod asset_upload;
pub(crate) mod block_model;
pub(crate) mod block_model_material;
pub(crate) mod block_tint;
mod celestial;
mod celestial_path;
mod directional_shadows;
mod dynamic_lights;
mod environment;
mod fog;
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
use directional_shadows::DirectionalShadowsPlugin;
use dynamic_lights::DynamicLightsPlugin;
use environment::EnvironmentPlugin;
use fog::FogPlugin;
use lighting::LightingPlugin;
use mesh_allocator_diagnostics::MeshAllocatorDiagnosticsPlugin;
use sky::SkyPlugin;
use sky_layers::SkyLayersPlugin;
use terrain_material::TerrainMaterial;

pub(crate) struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<TerrainMaterial>::default(),
            MaterialPlugin::<BlockModelMaterial>::default(),
        ))
        .add_systems(Startup, setup_block_model_assets)
        .add_plugins((
            AssetUploadPlugin,
            MeshAllocatorDiagnosticsPlugin,
            EnvironmentPlugin,
            LightingPlugin,
            DirectionalShadowsPlugin,
            DynamicLightsPlugin,
            FogPlugin,
            SkyPlugin,
            SkyLayersPlugin,
            CelestialPlugin,
        ));
    }
}
