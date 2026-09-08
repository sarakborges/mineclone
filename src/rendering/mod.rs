mod asset_upload;
mod celestial;
mod celestial_path;
mod environment;
mod fog;
mod lighting;
mod sky;
mod sky_layers;

use asset_upload::AssetUploadPlugin;
use bevy::prelude::*;
use celestial::CelestialPlugin;
use environment::EnvironmentPlugin;
use fog::FogPlugin;
use lighting::LightingPlugin;
use sky::SkyPlugin;
use sky_layers::SkyLayersPlugin;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AssetUploadPlugin,
            EnvironmentPlugin,
            LightingPlugin,
            FogPlugin,
            SkyPlugin,
            SkyLayersPlugin,
            CelestialPlugin,
        ));
    }
}
