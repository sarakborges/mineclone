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

use app::{game_state::GameState, pause_state::PauseState};
use bevy::{prelude::*, winit::WINIT_WINDOWS};
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
#[cfg(target_os = "windows")]
use winit::{
    dpi::PhysicalSize,
    platform::windows::{IconExtWindows, WindowExtWindows},
};
use winit::window::Icon;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Asteria".into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .init_state::<PauseState>()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.04)))
        .add_plugins((
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
        ))
        .add_systems(Update, set_window_icon)
        .run();
}

fn set_window_icon(mut icon_set: Local<bool>) {
    if *icon_set {
        return;
    }

    let mut applied = false;

    WINIT_WINDOWS.with(|windows| {
        let windows = windows.borrow();
        if windows.windows.is_empty() {
            return;
        }

        #[cfg(target_os = "windows")]
        let window_icon = Icon::from_resource(1, Some(PhysicalSize::new(32, 32)))
            .unwrap_or_else(|_| load_png_icon());
        #[cfg(not(target_os = "windows"))]
        let window_icon = load_png_icon();

        #[cfg(target_os = "windows")]
        let taskbar_icon = Icon::from_resource(1, Some(PhysicalSize::new(256, 256)))
            .unwrap_or_else(|_| load_png_icon());

        for window in windows.windows.values() {
            window.set_window_icon(Some(window_icon.clone()));

            #[cfg(target_os = "windows")]
            window.set_taskbar_icon(Some(taskbar_icon.clone()));

            applied = true;
        }
    });

    if applied {
        *icon_set = true;
    }
}

fn load_png_icon() -> Icon {
    let icon_rgba = image::load_from_memory(include_bytes!(
        "../assets/branding/asteria_icon.png"
    ))
    .expect("Asteria window icon should be a valid PNG")
    .into_rgba8();
    let (width, height) = icon_rgba.dimensions();

    Icon::from_rgba(icon_rgba.into_raw(), width, height)
        .expect("Asteria window icon should have valid RGBA dimensions")
}
