#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod content;
mod gameplay;
mod hud;
mod player;
mod rendering;
mod screens;
mod targeting;
mod ui;
mod voxel;
mod world;

use app::{
    crash_log::{install_crash_logger, write_caught_panic},
    game_state::GameState,
    pause_state::PauseState,
    runtime_paths::prepare_runtime_directory,
    window_icon::WindowIconPlugin,
};
use bevy::prelude::*;
use content::ContentPlugin;
use gameplay::GameplayPlugin;
use hud::HudPlugin;
use rendering::RenderingPlugin;
use screens::{
    loading_screen::LoadingScreenPlugin,
    pause_menu::PauseMenuPlugin,
    settings_screen::SettingsScreenPlugin,
    starting_screen::StartingScreenPlugin,
};
use targeting::block::BlockTargetingPlugin;
use ui::UiDesignSystemPlugin;
use world::WorldPlugin;

fn main() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_game));

    if let Err(payload) = result {
        write_caught_panic(payload.as_ref());
        std::panic::resume_unwind(payload);
    }
}

fn run_game() {
    prepare_runtime_directory();

    let mut app = App::new();
    app.add_plugins(
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
    ));

    // Install this after Bevy has built its plugins so another plugin cannot replace
    // our panic hook. Panics that happen before this point are still caught by main().
    install_crash_logger();
    app.run();
}
