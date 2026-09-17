mod loading_screen;
mod pause_menu;
mod settings_screen;
mod starting_screen;
mod world_selection;

use bevy::prelude::*;
use loading_screen::LoadingScreenPlugin;
use pause_menu::PauseMenuPlugin;
use settings_screen::SettingsScreenPlugin;
use starting_screen::StartingScreenPlugin;
use world_selection::WorldSelectionPlugin;

pub(crate) struct ScreensPlugin;

impl Plugin for ScreensPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SettingsScreenPlugin,
            StartingScreenPlugin,
            WorldSelectionPlugin,
            LoadingScreenPlugin,
            PauseMenuPlugin,
        ));
    }
}
