use bevy::prelude::*;

use super::theme;

pub const WIDTH: f32 = 52.0;
pub const HEIGHT: f32 = 30.0;
pub const THUMB_SIZE: f32 = 20.0;

const BORDER_WIDTH: f32 = 2.0;
const THUMB_INSET: f32 = 3.0;
const THUMB_ENABLED_LEFT: f32 =
    WIDTH - THUMB_SIZE - (BORDER_WIDTH * 2.0) - THUMB_INSET;

pub fn control(enabled: bool) -> impl Bundle {
    (
        Node {
            width: px(WIDTH),
            height: px(HEIGHT),
            flex_shrink: 0.0,
            position_type: PositionType::Relative,
            border: UiRect::all(px(BORDER_WIDTH)),
            ..default()
        },
        BackgroundColor(background(enabled, Interaction::None)),
        BorderColor::all(border(enabled)),
    )
}

pub fn thumb(enabled: bool) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(thumb_left(enabled)),
            top: px(THUMB_INSET),
            width: px(THUMB_SIZE),
            height: px(THUMB_SIZE),
            ..default()
        },
        BackgroundColor(theme::TEXT_PRIMARY),
        Pickable::IGNORE,
    )
}

pub const fn thumb_left(enabled: bool) -> f32 {
    if enabled {
        THUMB_ENABLED_LEFT
    } else {
        THUMB_INSET
    }
}

pub fn colors(enabled: bool, interaction: Interaction) -> (Color, Color) {
    (background(enabled, interaction), border(enabled))
}

fn border(enabled: bool) -> Color {
    if enabled {
        theme::BORDER_FOCUS
    } else {
        theme::BORDER
    }
}

fn background(enabled: bool, interaction: Interaction) -> Color {
    match (enabled, interaction) {
        (true, Interaction::Pressed) => theme::PURPLE,
        (true, Interaction::Hovered) => theme::PURPLE_HOVER,
        (true, Interaction::None) => theme::PURPLE_SOFT,
        (false, Interaction::Pressed) => theme::SURFACE_INSET,
        (false, Interaction::Hovered) => theme::SURFACE_ELEVATED,
        (false, Interaction::None) => theme::HUD_SURFACE,
    }
}
