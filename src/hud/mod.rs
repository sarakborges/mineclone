mod crosshair;
mod hotbar;
mod targeting;
mod time;
mod underwater;
mod world;

use bevy::prelude::*;
use crosshair::CrosshairPlugin;
use hotbar::HotbarHudPlugin;
use targeting::TargetHudPlugin;
use time::TimeHudPlugin;
use underwater::UnderwaterTintPlugin;
use world::WorldHudPlugin;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            UnderwaterTintPlugin,
            CrosshairPlugin,
            HotbarHudPlugin,
            TimeHudPlugin,
            WorldHudPlugin,
            TargetHudPlugin,
        ));
    }
}
