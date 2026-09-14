use bevy::prelude::*;

use crate::ui::surface;

pub(super) fn world_banner() -> impl Bundle {
    surface::hud_banner()
}
