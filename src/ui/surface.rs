use bevy::prelude::*;

use super::theme;

pub const HUD_BORDER_COLOR: Color = Color::srgba(0.70, 0.72, 0.82, 0.28);
pub const HUD_HOVER_BORDER_COLOR: Color = Color::srgba(0.78, 0.82, 0.96, 0.62);
pub const HUD_SELECTED_BORDER_COLOR: Color = theme::TEXT_PRIMARY;
pub const HUD_DANGER_BORDER_COLOR: Color = Color::srgba(0.94, 0.28, 0.34, 0.62);

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
    hud_surface(Node {
        padding: UiRect::axes(px(14), px(10)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    })
}

pub fn hud_strip() -> impl Bundle {
    hud_surface(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(4),
        padding: UiRect::all(px(4)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    })
}

pub fn hud_container(node: Node) -> impl Bundle {
    hud_surface(node)
}

pub fn hud_banner() -> impl Bundle {
    hud_surface(Node {
        width: px(520),
        max_width: percent(80),
        padding: UiRect::axes(px(28), px(7)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(1),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    })
}

pub fn hud_control_static(selected: bool) -> (Color, Color) {
    if selected {
        (
            Color::srgba(0.08, 0.07, 0.16, 0.94),
            HUD_SELECTED_BORDER_COLOR,
        )
    } else {
        (theme::HUD_SURFACE, HUD_BORDER_COLOR)
    }
}

pub fn hud_control_colors(interaction: Interaction, selected: bool) -> (Color, Color) {
    match (interaction, selected) {
        (Interaction::Pressed, true) | (Interaction::Hovered, true) => (
            Color::srgba(0.12, 0.10, 0.22, 0.98),
            HUD_SELECTED_BORDER_COLOR,
        ),
        (Interaction::None, true) => hud_control_static(true),
        (Interaction::Pressed, false) => (
            Color::srgba(0.09, 0.075, 0.17, 0.98),
            HUD_SELECTED_BORDER_COLOR.with_alpha(0.78),
        ),
        (Interaction::Hovered, false) => (
            Color::srgba(0.065, 0.052, 0.13, 0.94),
            HUD_HOVER_BORDER_COLOR,
        ),
        (Interaction::None, false) => hud_control_static(false),
    }
}

pub fn hud_danger_control_colors(interaction: Interaction) -> (Color, Color) {
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.31, 0.055, 0.075, 0.98),
            Color::srgba(1.0, 0.36, 0.40, 0.96),
        ),
        Interaction::Hovered => (
            Color::srgba(0.22, 0.045, 0.065, 0.94),
            Color::srgba(1.0, 0.32, 0.38, 0.82),
        ),
        Interaction::None => (
            theme::HUD_SURFACE,
            HUD_DANGER_BORDER_COLOR,
        ),
    }
}

fn hud_surface(node: Node) -> impl Bundle {
    (
        node,
        BackgroundColor(theme::HUD_SURFACE),
        BackgroundGradient::from(LinearGradient::to_bottom_right(vec![
            ColorStop::percent(Color::srgba(0.34, 0.18, 0.62, 0.16), 0.0),
            ColorStop::percent(Color::srgba(0.06, 0.045, 0.12, 0.05), 52.0),
            ColorStop::percent(Color::srgba(0.12, 0.34, 0.46, 0.10), 100.0),
        ])),
        BorderColor::all(HUD_BORDER_COLOR),
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
