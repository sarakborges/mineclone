use bevy::prelude::*;

use super::theme;

pub const BORDER_COLOR: Color = theme::BORDER;
pub const HOVER_BORDER_COLOR: Color = theme::BORDER_STRONG;
pub const SELECTED_BORDER_COLOR: Color = theme::BORDER_FOCUS;
pub const DANGER_BORDER_COLOR: Color = theme::BORDER_STRONG;

pub fn static_colors(selected: bool) -> (Color, Color) {
    if selected {
        (theme::PURPLE_SOFT, SELECTED_BORDER_COLOR)
    } else {
        (theme::HUD_SURFACE, BORDER_COLOR)
    }
}

pub fn colors(interaction: Interaction, selected: bool) -> (Color, Color) {
    match (interaction, selected) {
        (Interaction::Pressed, true) | (Interaction::Hovered, true) => {
            (theme::PURPLE_SOFT, SELECTED_BORDER_COLOR)
        }
        (Interaction::None, true) => static_colors(true),
        (Interaction::Pressed, false) => {
            (theme::SURFACE_INSET, SELECTED_BORDER_COLOR.with_alpha(0.78))
        }
        (Interaction::Hovered, false) => (theme::SURFACE_ELEVATED, HOVER_BORDER_COLOR),
        (Interaction::None, false) => static_colors(false),
    }
}

pub fn danger_colors(interaction: Interaction) -> (Color, Color) {
    match interaction {
        Interaction::Pressed => (
            Color::srgba(0.31, 0.055, 0.075, 0.98),
            Color::srgba(1.0, 0.36, 0.40, 0.96),
        ),
        Interaction::Hovered => (
            Color::srgba(0.22, 0.045, 0.065, 0.94),
            Color::srgba(1.0, 0.32, 0.38, 0.82),
        ),
        Interaction::None => (theme::HUD_SURFACE, DANGER_BORDER_COLOR),
    }
}

pub fn apply_colors(
    (background_color, border_color): (Color, Color),
    mut background: Mut<'_, BackgroundColor>,
    mut border: Mut<'_, BorderColor>,
) {
    if background.0 != background_color {
        background.0 = background_color;
    }
    let next_border = BorderColor::all(border_color);
    if *border != next_border {
        *border = next_border;
    }
}
