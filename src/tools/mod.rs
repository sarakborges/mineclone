mod brush;

use bevy::prelude::*;
use brush::BrushPlugin;

pub(crate) use crate::content::builtin_ids::DYED_PROPERTY_ID;
pub(crate) use brush::{BrushMode, BrushPaletteState};

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BrushPlugin);
    }
}
