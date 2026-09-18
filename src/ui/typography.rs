use bevy::{
    prelude::*,
    text::{FontWeight, LetterSpacing},
};

use super::theme;

fn ui_font(size: f32, weight: FontWeight) -> TextFont {
    TextFont {
        font: FontSource::SystemUi,
        font_size: FontSize::Px(size),
        weight,
        ..default()
    }
}

pub fn title(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(36.0, FontWeight::SEMIBOLD),
        LetterSpacing::Px(0.2),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn heading(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(26.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.1),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn setting_title(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(19.0, FontWeight::SEMIBOLD),
        LetterSpacing::Px(0.0),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn muted(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(15.0, FontWeight::NORMAL),
        LetterSpacing::Px(0.0),
        TextColor(theme::TEXT_MUTED),
    )
}

pub fn caption(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(13.0, FontWeight::NORMAL),
        LetterSpacing::Px(0.1),
        TextColor(theme::TEXT_SUBTLE),
    )
}

pub fn tooltip_shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::new(1.0, 1.0),
        color: Color::BLACK,
    }
}

pub fn crosshair_hint(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(13.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.1),
        TextColor(Color::WHITE),
        tooltip_shadow(),
    )
}

pub fn button_label(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(18.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.0),
        TextColor(theme::BUTTON_TEXT),
    )
}

pub fn hud(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(16.0, FontWeight::NORMAL),
        LetterSpacing::Px(0.0),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn inventory_category(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(13.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.0),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn hud_heading(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(24.0, FontWeight::SEMIBOLD),
        LetterSpacing::Px(0.1),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn hud_subheading(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(19.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.0),
        TextColor(theme::TEXT_PRIMARY),
    )
}
