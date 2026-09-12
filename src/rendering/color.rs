use bevy::prelude::*;

pub(crate) fn color_to_linear_vec4(color: Color) -> Vec4 {
    let color = color.to_linear();
    Vec4::new(color.red, color.green, color.blue, color.alpha)
}
