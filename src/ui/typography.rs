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
        ui_font(42.0, FontWeight::SEMIBOLD),
        LetterSpacing::Px(1.4),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn heading(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(28.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.8),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn muted(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(16.0, FontWeight::NORMAL),
        LetterSpacing::Px(0.35),
        TextColor(theme::TEXT_MUTED),
    )
}

pub fn caption(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(14.0, FontWeight::NORMAL),
        LetterSpacing::Px(0.8),
        TextColor(theme::TEXT_SUBTLE),
    )
}

pub fn button_label(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(20.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.9),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn hud(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(17.0, FontWeight::NORMAL),
        LetterSpacing::Px(0.25),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn hud_heading(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(27.0, FontWeight::SEMIBOLD),
        LetterSpacing::Px(0.9),
        TextColor(theme::TEXT_PRIMARY),
    )
}

pub fn hud_subheading(label: impl Into<String>) -> impl Bundle {
    (
        Text::new(label),
        ui_font(21.0, FontWeight::MEDIUM),
        LetterSpacing::Px(0.6),
        TextColor(theme::TEXT_PRIMARY),
    )
}
