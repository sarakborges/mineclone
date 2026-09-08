mod crosshair;
mod targeting;
mod time;
mod world;

use bevy::prelude::*;
use crosshair::CrosshairPlugin;
use targeting::TargetHudPlugin;
use time::TimeHudPlugin;
use world::WorldHudPlugin;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CrosshairPlugin,
            TimeHudPlugin,
            WorldHudPlugin,
            TargetHudPlugin,
        ));
    }
}
