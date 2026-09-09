mod asset_upload;
pub(crate) mod block_model;
mod celestial;
mod celestial_path;
mod environment;
mod fog;
mod sky;
mod sky_layers;
pub(crate) mod terrain_material;

use asset_upload::AssetUploadPlugin;
use bevy::prelude::*;
use celestial::CelestialPlugin;
use environment::EnvironmentPlugin;
use fog::FogPlugin;
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
                FogPlugin,
                SkyPlugin,
                SkyLayersPlugin,
                CelestialPlugin,
            ));
    }
}
