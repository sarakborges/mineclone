use bevy::{
    ecs::system::NonSendMarker,
    prelude::*,
    winit::WINIT_WINDOWS,
};
#[cfg(target_os = "windows")]
use winit::{
    dpi::PhysicalSize,
    platform::windows::{IconExtWindows, WindowExtWindows},
};
use winit::window::Icon;

pub struct WindowIconPlugin;

impl Plugin for WindowIconPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, set_window_icon);
    }
}

fn set_window_icon(mut icon_set: Local<bool>, _main_thread: NonSendMarker) {
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
        "../../assets/branding/asteria_icon.png"
    ))
    .expect("Asteria window icon should be a valid PNG")
    .into_rgba8();
    let (width, height) = icon_rgba.dimensions();

    Icon::from_rgba(icon_rgba.into_raw(), width, height)
        .expect("Asteria window icon should have valid RGBA dimensions")
}
