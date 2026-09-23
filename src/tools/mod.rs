mod artisans_kit;
mod brush;
mod carpenters_axe;
mod shears;
mod structure_tool;

use artisans_kit::ArtisansKitPlugin;
use bevy::prelude::*;
use brush::BrushPlugin;
use carpenters_axe::CarpentersAxePlugin;
use shears::ShearsPlugin;
use structure_tool::StructureToolPlugin;

pub(crate) use brush::BrushMode;

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ArtisansKitPlugin,
            BrushPlugin,
            CarpentersAxePlugin,
            ShearsPlugin,
            StructureToolPlugin,
        ));
    }
}
