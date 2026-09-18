use bevy::prelude::*;

// Ore-UI-inspired design tokens: neutral, layered surfaces, crisp borders,
// compact spacing, and restrained accent color. Keep screen code dependent on
// these tokens instead of inventing per-screen chrome.

pub const SCREEN_BACKGROUND: Color = Color::srgb(0.055, 0.055, 0.055);
pub const OVERLAY: Color = Color::srgba(0.02, 0.02, 0.02, 0.72);
pub const FROSTED_SURFACE: Color = Color::srgb(0.105, 0.105, 0.105);
pub const HUD_SURFACE: Color = Color::srgba(0.075, 0.075, 0.075, 0.94);

pub const SURFACE_ELEVATED: Color = Color::srgb(0.125, 0.125, 0.125);
pub const SURFACE_INSET: Color = Color::srgb(0.045, 0.045, 0.045);
pub const BORDER: Color = Color::srgba(0.72, 0.72, 0.72, 0.34);
pub const BORDER_STRONG: Color = Color::srgba(0.90, 0.90, 0.90, 0.58);
pub const BORDER_FOCUS: Color = Color::srgba(0.92, 0.92, 0.92, 0.90);

pub const TEXT_PRIMARY: Color = Color::srgb(0.96, 0.96, 0.96);
pub const TEXT_MUTED: Color = Color::srgb(0.72, 0.72, 0.72);
pub const TEXT_SUBTLE: Color = Color::srgba(0.72, 0.72, 0.72, 0.64);

pub const ACCENT: Color = Color::srgb(0.46, 0.78, 0.58);
pub const ACCENT_SOFT: Color = Color::srgba(0.46, 0.78, 0.58, 0.20);

pub const SLIDER_TRACK: Color = Color::srgb(0.16, 0.16, 0.16);
pub const SLIDER_THUMB: Color = Color::srgb(0.90, 0.90, 0.90);

pub fn cosmic_background_gradient() -> BackgroundGradient {
    // Kept as the existing API so screens do not need to change.
    // The new design system intentionally uses a flat, restrained backdrop.
    BackgroundGradient::from(LinearGradient::to_bottom_right(vec![
        ColorStop::auto(Color::srgba(1.0, 1.0, 1.0, 0.025)),
        ColorStop::auto(Color::srgba(1.0, 1.0, 1.0, 0.0)),
    ]))
}

pub fn frosted_surface_gradient() -> BackgroundGradient {
    // Subtle top-edge lift instead of the previous purple/cyan glow.
    BackgroundGradient::from(LinearGradient::to_bottom(vec![
        ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.035), 0.0),
        ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.0), 16.0),
        ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.025), 100.0),
    ]))
}
