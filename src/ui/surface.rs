use bevy::prelude::*;

use super::theme;

const CONTROL_BORDER_WIDTH: f32 = 2.0;

pub fn settings_content() -> impl Bundle {
    frosted_surface(Node {
        width: percent(100),
        padding: UiRect::all(px(18)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        border: UiRect::all(px(CONTROL_BORDER_WIDTH)),
        ..default()
    })
}

pub fn hud_container(node: Node) -> impl Bundle {
    hud_surface(node)
}

fn hud_surface(node: Node) -> impl Bundle {
    (
        node,
        BackgroundColor(theme::HUD_SURFACE),
        theme::frosted_surface_gradient(),
        BorderColor::all(theme::BORDER),
        BoxShadow(vec![ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.30),
            x_offset: px(0),
            y_offset: px(3),
            spread_radius: px(0),
            blur_radius: px(10),
        }]),
    )
}

fn frosted_surface(node: Node) -> impl Bundle {
    (
        node,
        BackgroundColor(theme::FROSTED_SURFACE),
        theme::frosted_surface_gradient(),
        BorderColor::all(theme::BORDER),
        BoxShadow(vec![ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.38),
            x_offset: px(0),
            y_offset: px(8),
            spread_radius: px(0),
            blur_radius: px(18),
        }]),
    )
}
