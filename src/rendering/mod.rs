mod fog;
mod lighting;

use bevy::prelude::*;
use fog::FogPlugin;
use lighting::LightingPlugin;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((LightingPlugin, FogPlugin));
    }
}
