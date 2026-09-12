mod block_icon;
mod crosshair;
mod hotbar;
mod inventory;
mod targeting;
mod time;
mod underwater;
mod world;

use bevy::prelude::*;
use block_icon::BlockIconMaterial;
use crosshair::CrosshairPlugin;
use hotbar::HotbarHudPlugin;
use inventory::InventoryHudPlugin;
use targeting::TargetHudPlugin;
use time::TimeHudPlugin;
use underwater::UnderwaterTintPlugin;
use world::WorldHudPlugin;

pub(crate) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<BlockIconMaterial>::default())
            .add_plugins((
                UnderwaterTintPlugin,
                CrosshairPlugin,
                HotbarHudPlugin,
                InventoryHudPlugin,
                TimeHudPlugin,
                WorldHudPlugin,
                TargetHudPlugin,
            ));
    }
}
