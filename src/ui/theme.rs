use bevy::prelude::*;

// Asteria/Ore-UI tokens: dark violet surfaces, crisp rectangular controls,
// restrained purple framing, and Minecraft-style green/red semantic states.
pub const SCREEN_BACKGROUND: Color = Color::srgb(0.045, 0.043, 0.060);
pub const FROSTED_SURFACE: Color = Color::srgb(0.115, 0.105, 0.145);
pub const HUD_SURFACE: Color = Color::srgba(0.070, 0.064, 0.090, 0.96);
pub const SURFACE_ELEVATED: Color = Color::srgb(0.155, 0.140, 0.190);
pub const SURFACE_INSET: Color = Color::srgb(0.038, 0.034, 0.052);
pub const BORDER: Color = Color::srgba(0.55, 0.50, 0.66, 0.62);
pub const BORDER_STRONG: Color = Color::srgba(0.82, 0.78, 0.90, 0.88);
pub const BORDER_FOCUS: Color = Color::srgba(0.72, 0.58, 0.98, 0.98);
pub const TEXT_PRIMARY: Color = Color::srgb(0.94, 0.94, 0.96);
pub const TEXT_MUTED: Color = Color::srgb(0.70, 0.68, 0.76);
pub const TEXT_SUBTLE: Color = Color::srgba(0.70, 0.68, 0.76, 0.68);
pub const PURPLE: Color = Color::srgb(0.48, 0.30, 0.72);
pub const PURPLE_HOVER: Color = Color::srgb(0.58, 0.38, 0.84);
pub const PURPLE_SOFT: Color = Color::srgba(0.42, 0.27, 0.62, 0.72);
pub const DANGER: Color = Color::srgb(0.78, 0.20, 0.20);
pub const DANGER_HOVER: Color = Color::srgb(0.86, 0.28, 0.28);
pub const DANGER_PRESSED: Color = Color::srgb(0.62, 0.14, 0.14);
pub const SLIDER_TRACK: Color = Color::srgb(0.20, 0.18, 0.24);
pub const SLIDER_THUMB: Color = Color::srgb(0.88, 0.88, 0.90);

pub fn cosmic_background_gradient() -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::to_bottom_right(vec![
        ColorStop::auto(Color::srgba(0.34, 0.22, 0.52, 0.10)),
        ColorStop::auto(Color::srgba(0.34, 0.22, 0.52, 0.0)),
    ]))
}
pub fn frosted_surface_gradient() -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::to_bottom(vec![
        ColorStop::percent(Color::srgba(0.58, 0.42, 0.78, 0.08), 0.0),
        ColorStop::percent(Color::srgba(0.58, 0.42, 0.78, 0.0), 20.0),
        ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.035), 100.0),
    ]))
}
