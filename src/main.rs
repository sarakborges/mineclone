#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod content;
mod creatures;
mod gameplay;
mod hud;
mod localization;
mod player;
mod rendering;
mod screens;
mod targeting;
mod tools;
mod ui;
mod voxel;
mod world;

#[cfg(debug_assertions)]
#[expect(
    unused_imports,
    clippy::single_component_path_imports,
    reason = "the debug import intentionally enables Bevy dynamic linking"
)]
use bevy_dylib;

use app::{
    crash_log::{install_crash_logger, mark_clean_shutdown, write_caught_panic},
    game_config::GameConfigPlugin,
    game_state::GameState,
    pause_state::PauseState,
    runtime_paths::prepare_runtime_directory,
    window_icon::WindowIconPlugin,
};
use bevy::{
    app::{TaskPoolOptions, TaskPoolPlugin},
    prelude::*,
};
use content::ContentPlugin;
use creatures::CreaturesPlugin;
use gameplay::GameplayPlugin;
use hud::HudPlugin;
use localization::LocalizationPlugin;
use rendering::RenderingPlugin;
use screens::ScreensPlugin;
use targeting::block::BlockTargetingPlugin;
use tools::ToolsPlugin;
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
                .set(TaskPoolPlugin {
                    task_pool_options: voxel_task_pool_options(),
                })
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
            GameConfigPlugin,
            UiDesignSystemPlugin,
            LocalizationPlugin,
            ContentPlugin,
            ScreensPlugin,
            WorldPlugin,
            GameplayPlugin,
            CreaturesPlugin,
            RenderingPlugin,
            BlockTargetingPlugin,
            ToolsPlugin,
            HudPlugin,
        ))
        .run();
}

fn voxel_task_pool_options() -> TaskPoolOptions {
    let mut options = TaskPoolOptions::default();
    options.io.percent = 0.10;
    options.io.max_threads = 2;
    options.async_compute.percent = 0.50;
    options.async_compute.max_threads = 8;
    options
}
