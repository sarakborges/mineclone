mod chunk;
mod crosshair;
mod targeting;
mod world;

use bevy::prelude::*;
use chunk::ChunkHudPlugin;
use crosshair::CrosshairPlugin;
use targeting::TargetHudPlugin;
use world::WorldHudPlugin;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CrosshairPlugin,
            ChunkHudPlugin,
            WorldHudPlugin,
            TargetHudPlugin,
        ));
    }
}
