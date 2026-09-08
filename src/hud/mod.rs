mod crosshair;
mod player;
mod targeting;

use bevy::prelude::*;
use crosshair::CrosshairPlugin;
use player::PlayerHudPlugin;
use targeting::TargetHudPlugin;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CrosshairPlugin, PlayerHudPlugin, TargetHudPlugin));
    }
}
