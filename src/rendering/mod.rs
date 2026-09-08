mod environment;
mod fog;
mod lighting;
mod sky;

use bevy::prelude::*;
use environment::EnvironmentPlugin;
use fog::FogPlugin;
use lighting::LightingPlugin;
use sky::SkyPlugin;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((EnvironmentPlugin, LightingPlugin, FogPlugin, SkyPlugin));
    }
}
