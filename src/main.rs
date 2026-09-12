#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod content;
mod gameplay;
mod hud;
mod localization;
mod player;
mod rendering;
mod screens;
mod targeting;
mod ui;
mod voxel;
mod world;

use app::{
    crash_log::{install_crash_logger, mark_clean_shutdown, write_caught_panic},
    game_state::GameState,
    pause_state::PauseState,
    runtime_paths::prepare_runtime_directory,
    window_icon::WindowIconPlugin,
};
use bevy::prelude::*;
use content::ContentPlugin;
use gameplay::GameplayPlugin;
use hud::HudPlugin;
use localization::LocalizationPlugin;
use rendering::RenderingPlugin;
use screens::{
    loading_screen::LoadingScreenPlugin, pause_menu::PauseMenuPlugin,
    settings_screen::SettingsScreenPlugin, starting_screen::StartingScreenPlugin,
};
use targeting::block::BlockTargetingPlugin;
use ui::UiDesignSystemPlugin;
use world::WorldPlugin;

fn main() {
    install_crash_logger();

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_game)) {
        Ok(()) => mark_clean_shutdown(),
        Err(payload) => {
            write_caught_panic(payload.as_ref());
            std::panic::resume_unwind(payload);
        }
    }
}

fn run_game() {
    prepare_runtime_directory();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Asteria".into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .init_state::<GameState>()
        .init_state::<PauseState>()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.04)))
        .add_plugins((
            WindowIconPlugin,
            UiDesignSystemPlugin,
            LocalizationPlugin,
            ContentPlugin,
            SettingsScreenPlugin,
            StartingScreenPlugin,
            LoadingScreenPlugin,
            PauseMenuPlugin,
            WorldPlugin,
            GameplayPlugin,
            RenderingPlugin,
            BlockTargetingPlugin,
            HudPlugin,
        ))
        .run();
}
