mod brush;
mod artisans_kit;
mod shears;
mod structure_tool;

use bevy::prelude::*;
use brush::BrushPlugin;
use artisans_kit::ArtisansKitPlugin;
use shears::ShearsPlugin;
use structure_tool::StructureToolPlugin;

pub(crate) use brush::BrushMode;

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BrushPlugin, ArtisansKitPlugin, ShearsPlugin, StructureToolPlugin));
    }
}
