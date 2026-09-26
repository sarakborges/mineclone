use bevy::prelude::*;

pub(crate) fn color_to_linear_vec4(color: Color) -> Vec4 {
    let color = color.to_linear();
    Vec4::new(color.red, color.green, color.blue, color.alpha)
}

pub(crate) const MATERIAL_TINT_RGB_LEVELS: u8 = 31;

pub(crate) fn quantize_srgba(color: Color, rgb_levels: u8) -> (Color, [u8; 4]) {
    debug_assert!(rgb_levels > 0);
    let rgba = color.to_srgba();
    let levels = f32::from(rgb_levels);
    let quantize_rgb =
        |value: f32| (value.clamp(0.0, 1.0) * levels).round() as u8;
    let red = quantize_rgb(rgba.red);
    let green = quantize_rgb(rgba.green);
    let blue = quantize_rgb(rgba.blue);
    let alpha = (rgba.alpha.clamp(0.0, 1.0) * 255.0).round() as u8;

    (
        Color::srgba(
            f32::from(red) / levels,
            f32::from(green) / levels,
            f32::from(blue) / levels,
            f32::from(alpha) / 255.0,
        ),
        [red, green, blue, alpha],
    )
}
