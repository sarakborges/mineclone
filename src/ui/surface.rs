use bevy::prelude::*;

use super::theme;

pub fn modal_panel() -> impl Bundle {
    frosted_surface(Node {
        width: px(560),
        padding: UiRect::all(px(34)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(20),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    })
}

pub fn settings_section() -> impl Bundle {
    frosted_surface(Node {
        width: percent(100),
        padding: UiRect::all(px(30)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(18),
        border_radius: BorderRadius::all(px(8)),
        ..default()
    })
}

pub fn settings_sidebar(width: f32) -> impl Bundle {
    frosted_surface(Node {
        width: px(width),
        height: percent(100),
        min_height: px(0),
        padding: UiRect::all(px(16)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        border_radius: BorderRadius::all(px(8)),
        ..default()
    })
}

pub fn settings_content() -> impl Bundle {
    frosted_surface(Node {
        flex_grow: 1.0,
        height: percent(100),
        min_width: px(0),
        min_height: px(0),
        padding: UiRect::all(px(18)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        border_radius: BorderRadius::all(px(8)),
        ..default()
    })
}

pub fn hud_panel() -> impl Bundle {
    (
        Node {
            padding: UiRect::axes(px(14), px(10)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(theme::HUD_SURFACE),
        BackgroundGradient::from(LinearGradient::to_bottom_right(vec![
            ColorStop::percent(Color::srgba(0.34, 0.18, 0.62, 0.18), 0.0),
            ColorStop::percent(Color::srgba(0.06, 0.045, 0.12, 0.06), 52.0),
            ColorStop::percent(Color::srgba(0.12, 0.34, 0.46, 0.12), 100.0),
        ])),
        BoxShadow(vec![ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.32),
            x_offset: px(0),
            y_offset: px(5),
            spread_radius: px(0),
            blur_radius: px(16),
        }]),
    )
}

fn frosted_surface(node: Node) -> impl Bundle {
    (
        node,
        BackgroundColor(theme::FROSTED_SURFACE),
        theme::frosted_surface_gradient(),
        BoxShadow(vec![
            ShadowStyle {
                color: Color::srgba(0.0, 0.0, 0.0, 0.44),
                x_offset: px(0),
                y_offset: px(12),
                spread_radius: px(0),
                blur_radius: px(30),
            },
            ShadowStyle {
                color: Color::srgba(0.42, 0.24, 0.92, 0.12),
                x_offset: px(0),
                y_offset: px(0),
                spread_radius: px(-6),
                blur_radius: px(26),
            },
        ]),
    )
}
