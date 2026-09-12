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

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct HudSettings {
    display_tooltips: bool,
}

impl Default for HudSettings {
    fn default() -> Self {
        Self {
            display_tooltips: true,
        }
    }
}

impl HudSettings {
    pub(crate) const fn display_tooltips(&self) -> bool {
        self.display_tooltips
    }

    pub(crate) fn set_display_tooltips(&mut self, display_tooltips: bool) {
        self.display_tooltips = display_tooltips;
    }
}

pub(crate) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSettings>()
            .add_plugins(UiMaterialPlugin::<BlockIconMaterial>::default())
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
