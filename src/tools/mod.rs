mod brush;
mod chisel;
mod shears;

use bevy::prelude::*;
use brush::BrushPlugin;
use chisel::ChiselPlugin;
use shears::ShearsPlugin;

pub(crate) use brush::{BrushMode, BrushPaletteState};

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BrushPlugin, ChiselPlugin, ShearsPlugin));
    }
}
