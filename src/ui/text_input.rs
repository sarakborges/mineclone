use bevy::{
    prelude::*,
    text::{EditableText, FontWeight, TextCursorStyle},
};

use super::theme;

pub(crate) const INPUT_FILL: Color = theme::SURFACE_INSET;
pub(crate) const INPUT_BORDER: Color = theme::BORDER;
pub(crate) const INPUT_PADDING_X: f32 = 14.0;
pub(crate) const INPUT_EDITOR_HEIGHT: f32 = 22.0;

pub(crate) fn centered_text_top(frame_height: f32) -> f32 {
    (frame_height - INPUT_EDITOR_HEIGHT) * 0.5
}

pub(crate) fn input_border(focused: bool) -> Color {
    if focused {
        theme::BORDER_FOCUS
    } else {
        INPUT_BORDER
    }
}

pub(crate) fn frame_surface(focused: bool) -> impl Bundle {
    (
        BackgroundColor(INPUT_FILL),
        BorderColor::all(input_border(focused)),
    )
}

pub(crate) fn editor_style(font_size: f32, weight: FontWeight) -> impl Bundle {
    (
        TextFont {
            font: FontSource::SystemUi,
            font_size: FontSize::Px(font_size),
            weight,
            ..default()
        },
        TextColor(theme::TEXT_PRIMARY),
        TextCursorStyle {
            color: theme::TEXT_PRIMARY,
            ..default()
        },
        TextLayout::no_wrap(),
    )
}

pub(crate) fn editable_value(input: &EditableText) -> String {
    input.value().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_editor_reads_unicode_and_clears() {
        let mut input = EditableText::new("floresta encantada 🌿");
        assert_eq!(editable_value(&input), "floresta encantada 🌿");
        input.clear();
        assert_eq!(editable_value(&input), "");
    }

    #[test]
    fn input_focus_changes_only_border() {
        assert_eq!(input_border(false), INPUT_BORDER);
        assert_eq!(input_border(true), theme::BORDER_FOCUS);
    }

    #[test]
    fn line_box_is_centered_within_search_field() {
        assert_eq!(centered_text_top(40.0), 9.0);
    }
}
