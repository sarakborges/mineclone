#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod content;
mod creatures;
mod entity;
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
mod world_items;
mod world_objects;

#[cfg(debug_assertions)]
#[expect(
    unused_imports,
    clippy::single_component_path_imports,
    reason = "the debug import intentionally enables Bevy dynamic linking"
)]
use bevy_dylib;

use app::{
    controls_state::ControlsState,
    crash_log::{install_crash_logger, log_system_event, mark_clean_shutdown, write_caught_panic},
    game_config::GameConfigPlugin,
    game_state::GameState,
    pause_state::PauseState,
    runtime_paths::prepare_runtime_directory,
    window_icon::WindowIconPlugin,
};
#[cfg(target_os = "windows")]
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};
use bevy::{
    app::{TaskPoolOptions, TaskPoolPlugin},
    prelude::*,
    window::PresentMode,
};
use content::ContentPlugin;
use creatures::CreaturesPlugin;
use gameplay::GameplayPlugin;
use hud::HudPlugin;
use localization::LocalizationPlugin;
use rendering::RenderingPlugin;
use screens::ScreensPlugin;
use targeting::{biome_tint::BiomeTintInteractionPlugin, block::BlockTargetingPlugin};
use tools::ToolsPlugin;
use ui::UiDesignSystemPlugin;
use voxel::block_gravity::BlockGravityPlugin;
use world::WorldPlugin;
use world_items::WorldItemsPlugin;
use world_objects::WorldObjectsPlugin;

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
    log_system_event(format!(
        "app.start debug={} backend_override={} present_mode={:?} io_pool_percent=0.10 io_pool_max_threads=2 async_compute_percent=0.50 async_compute_max_threads=8",
        cfg!(debug_assertions),
        std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "<default>".to_owned()),
        primary_present_mode(),
    ));

    let default_plugins = DefaultPlugins
        .set(TaskPoolPlugin {
            task_pool_options: voxel_task_pool_options(),
        })
        .set(ImagePlugin::default_nearest())
        .set(WindowPlugin {
            // World exit owns durability. The OS close button must not
            // destroy the window before the active world is saved.
            close_when_requested: false,
            primary_window: Some(primary_window()),
            ..default()
        });
    #[cfg(target_os = "windows")]
    let default_plugins = default_plugins.set(RenderPlugin {
        render_creation: windows_wgpu_settings().into(),
        ..default()
    });

    App::new()
        .add_plugins(default_plugins)
        .init_state::<GameState>()
        .init_state::<PauseState>()
        .init_state::<ControlsState>()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.04)))
        .add_systems(OnEnter(GameState::StartingScreen), log_game_state)
        .add_systems(OnEnter(GameState::NewWorld), log_game_state)
        .add_systems(OnEnter(GameState::WorldSelection), log_game_state)
        .add_systems(OnEnter(GameState::Loading), log_game_state)
        .add_systems(OnEnter(GameState::Gameplay), log_game_state)
        .add_systems(OnEnter(PauseState::Running), log_pause_state)
        .add_systems(OnEnter(PauseState::Paused), log_pause_state)
        .add_plugins((
            WindowIconPlugin,
            GameConfigPlugin,
            UiDesignSystemPlugin,
            LocalizationPlugin,
            ContentPlugin,
            ScreensPlugin,
            WorldPlugin,
            GameplayPlugin,
        ))
        .add_plugins((
            CreaturesPlugin,
            RenderingPlugin,
            WorldObjectsPlugin,
            WorldItemsPlugin,
            BlockGravityPlugin,
            BlockTargetingPlugin,
            ToolsPlugin,
            HudPlugin,
        ))
        .add_plugins(BiomeTintInteractionPlugin)
        .add_plugins(targeting::portal_activation_plugin())
        .run();
}

fn log_game_state(state: Res<State<GameState>>) {
    log_system_event(format!("state.game entered={:?}", state.get()));
}

fn log_pause_state(state: Res<State<PauseState>>) {
    log_system_event(format!("state.pause entered={:?}", state.get()));
}

#[cfg(target_os = "windows")]
fn windows_wgpu_settings() -> WgpuSettings {
    let mut settings = WgpuSettings::default();

    // Bevy 0.19 / wgpu 29 currently has a Vulkan VRAM-residency regression on
    // Windows. Prefer DX12 unless the user explicitly chose a backend through
    // WGPU_BACKEND, preserving the standard wgpu escape hatch for debugging or
    // unsupported hardware.
    if std::env::var_os("WGPU_BACKEND").is_none() {
        settings.backends = Some(Backends::DX12);
    }

    settings
}

fn primary_window() -> Window {
    let mut window = Window {
        title: "Asteria".into(),
        mode: bevy::window::WindowMode::Windowed,
        present_mode: primary_present_mode(),
        ..default()
    };
    window.set_maximized(true);
    window
}

fn primary_present_mode() -> PresentMode {
    // The Windows renderer is explicitly DX12. Mailbox is supported by the
    // DX11/12 presentation path and avoids FIFO's hard 60 -> 30 FPS step when
    // a frame narrowly misses a vblank, while still presenting without tearing.
    #[cfg(target_os = "windows")]
    {
        if std::env::var_os("WGPU_BACKEND").is_none() {
            PresentMode::Mailbox
        } else {
            PresentMode::AutoVsync
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        PresentMode::AutoVsync
    }
}

fn voxel_task_pool_options() -> TaskPoolOptions {
    let mut options = TaskPoolOptions::default();
    options.io.percent = 0.10;
    options.io.max_threads = 2;
    options.async_compute.percent = 0.50;
    options.async_compute.max_threads = 8;
    options
}
