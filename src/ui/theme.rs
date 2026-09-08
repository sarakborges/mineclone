use bevy::prelude::*;

pub const SCREEN_BACKGROUND: Color = Color::srgb(0.018, 0.014, 0.036);
pub const OVERLAY: Color = Color::srgba(0.012, 0.010, 0.028, 0.72);
pub const FROSTED_SURFACE: Color = Color::srgba(0.035, 0.027, 0.072, 0.82);
pub const HUD_SURFACE: Color = Color::srgba(0.025, 0.020, 0.055, 0.78);

pub const TEXT_PRIMARY: Color = Color::srgb(0.95, 0.96, 1.0);
pub const TEXT_MUTED: Color = Color::srgb(0.68, 0.66, 0.78);
pub const TEXT_SUBTLE: Color = Color::srgba(0.73, 0.75, 0.86, 0.58);

pub const CYAN_GLOW: Color = Color::srgba(0.25, 0.76, 1.0, 0.20);

pub const SLIDER_TRACK: Color = Color::srgba(0.12, 0.09, 0.22, 0.90);
pub const SLIDER_THUMB: Color = Color::srgb(0.78, 0.83, 0.98);

pub fn cosmic_background_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![
        RadialGradient {
            position: UiPosition::CENTER,
            shape: RadialGradientShape::Circle(vh(72)),
            stops: vec![
                ColorStop::auto(Color::srgba(0.30, 0.12, 0.62, 0.28)),
                ColorStop::auto(Color::srgba(0.18, 0.08, 0.40, 0.14)),
                ColorStop::auto(Color::srgba(0.08, 0.03, 0.18, 0.0)),
            ],
            ..default()
        }
        .into(),
        LinearGradient::to_top_right(vec![
            ColorStop::auto(Color::srgba(0.04, 0.24, 0.34, 0.14)),
            ColorStop::auto(Color::srgba(0.02, 0.06, 0.12, 0.0)),
            ColorStop::auto(Color::srgba(0.20, 0.05, 0.30, 0.10)),
        ])
        .into(),
    ])
}

pub fn frosted_surface_gradient() -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::to_bottom_right(vec![
        ColorStop::auto(Color::srgba(0.43, 0.24, 0.78, 0.12)),
        ColorStop::auto(Color::srgba(0.08, 0.07, 0.15, 0.02)),
        ColorStop::auto(Color::srgba(0.20, 0.55, 0.68, 0.07)),
    ]))
}
