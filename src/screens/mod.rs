mod loading_screen;
mod pause_menu;
mod settings_screen;
mod starting_screen;

use bevy::prelude::*;
use loading_screen::LoadingScreenPlugin;
use pause_menu::PauseMenuPlugin;
use settings_screen::SettingsScreenPlugin;
use starting_screen::StartingScreenPlugin;

pub(crate) struct ScreensPlugin;

impl Plugin for ScreensPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SettingsScreenPlugin,
            StartingScreenPlugin,
            LoadingScreenPlugin,
            PauseMenuPlugin,
        ));
    }
}
