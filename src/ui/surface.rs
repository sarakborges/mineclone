use bevy::prelude::*;

use super::theme;

pub const HUD_BORDER_COLOR: Color = theme::BORDER;
const CONTROL_BORDER_WIDTH: u32 = 2;
pub const HUD_HOVER_BORDER_COLOR: Color = theme::BORDER_STRONG;
pub const HUD_SELECTED_BORDER_COLOR: Color = theme::ACCENT;
pub const HUD_DANGER_BORDER_COLOR: Color = Color::srgba(0.92, 0.28, 0.30, 0.72);

pub fn modal_panel() -> impl Bundle {
    frosted_surface(Node { width: px(560), padding: UiRect::all(px(28)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(18), border: UiRect::all(px(CONTROL_BORDER_WIDTH)), ..default() })
}

pub fn settings_content() -> impl Bundle {
    frosted_surface(Node { flex_grow: 1.0, height: percent(100), min_width: px(0), min_height: px(0), padding: UiRect::all(px(18)), flex_direction: FlexDirection::Column, align_items: AlignItems::Stretch, border: UiRect::all(px(1)), ..default() })
}

pub fn hud_container(node: Node) -> impl Bundle { hud_surface(node) }

pub fn hud_control_static(selected: bool) -> (Color, Color) {
    if selected { (theme::PURPLE_SOFT, HUD_SELECTED_BORDER_COLOR) } else { (theme::HUD_SURFACE, HUD_BORDER_COLOR) }
}

pub fn hud_control_colors(interaction: Interaction, selected: bool) -> (Color, Color) {
    match (interaction, selected) {
        (Interaction::Pressed, true) | (Interaction::Hovered, true) => (theme::SURFACE_INSET, HUD_SELECTED_BORDER_COLOR),
        (Interaction::None, true) => hud_control_static(true),
        (Interaction::Pressed, false) => (theme::SURFACE_INSET, HUD_SELECTED_BORDER_COLOR.with_alpha(0.78)),
        (Interaction::Hovered, false) => (theme::SURFACE_ELEVATED, HUD_HOVER_BORDER_COLOR),
        (Interaction::None, false) => hud_control_static(false),
    }
}

pub fn hud_danger_control_colors(interaction: Interaction) -> (Color, Color) {
    match interaction {
        Interaction::Pressed => (Color::srgba(0.31, 0.055, 0.075, 0.98), Color::srgba(1.0, 0.36, 0.40, 0.96)),
        Interaction::Hovered => (Color::srgba(0.22, 0.045, 0.065, 0.94), Color::srgba(1.0, 0.32, 0.38, 0.82)),
        Interaction::None => (theme::HUD_SURFACE, HUD_DANGER_BORDER_COLOR),
    }
}

pub(crate) fn apply_control_colors((background_color, border_color): (Color, Color), mut background: Mut<'_, BackgroundColor>, mut border: Mut<'_, BorderColor>) {
    if background.0 != background_color { background.0 = background_color; }
    let next_border = BorderColor::all(border_color);
    if *border != next_border { *border = next_border; }
}

fn hud_surface(node: Node) -> impl Bundle {
    (node, BackgroundColor(theme::HUD_SURFACE), theme::frosted_surface_gradient(), BorderColor::all(HUD_BORDER_COLOR), BoxShadow(vec![ShadowStyle { color: Color::srgba(0.0, 0.0, 0.0, 0.30), x_offset: px(0), y_offset: px(3), spread_radius: px(0), blur_radius: px(10) }]))
}

fn frosted_surface(node: Node) -> impl Bundle {
    (node, BackgroundColor(theme::FROSTED_SURFACE), theme::frosted_surface_gradient(), BorderColor::all(theme::BORDER), BoxShadow(vec![ShadowStyle { color: Color::srgba(0.0, 0.0, 0.0, 0.38), x_offset: px(0), y_offset: px(8), spread_radius: px(0), blur_radius: px(18) }]))
}
