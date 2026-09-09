mod asset_upload;
pub(crate) mod block_model;
mod celestial;
mod celestial_path;
mod directional_shadows;
mod dynamic_lights;
mod environment;
mod fog;
mod lighting;
mod sky;
mod sky_layers;
pub(crate) mod terrain_material;

use asset_upload::AssetUploadPlugin;
use bevy::prelude::*;
use celestial::CelestialPlugin;
use directional_shadows::DirectionalShadowsPlugin;
use dynamic_lights::DynamicLightsPlugin;
use environment::EnvironmentPlugin;
use fog::FogPlugin;
use lighting::LightingPlugin;
use sky::SkyPlugin;
use sky_layers::SkyLayersPlugin;
use terrain_material::TerrainMaterial;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_plugins((
                AssetUploadPlugin,
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
