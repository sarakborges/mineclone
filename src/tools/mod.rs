mod brush;
mod chisel;

use bevy::prelude::*;
use brush::BrushPlugin;
use chisel::ChiselPlugin;

pub(crate) use brush::{BrushMode, BrushPaletteState};

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BrushPlugin, ChiselPlugin));
    }
}
