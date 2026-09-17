use bevy::{prelude::*, text::EditableText};

use super::theme;

/// Shared input chrome: do not style the chat, numeric inputs and search field separately.
pub(crate) const INPUT_FILL: Color = Color::srgba(0.045, 0.035, 0.09, 0.88);
pub(crate) const INPUT_BORDER: Color = Color::srgba(0.43, 0.36, 0.68, 0.72);
pub(crate) const INPUT_RADIUS: f32 = 7.0;
pub(crate) const INPUT_PADDING_X: f32 = 14.0;
/// Explicit editor line box: a full-height EditableText lays out its own glyphs
/// at the top; centering the full-height node does not center its text/caret.
pub(crate) const INPUT_EDITOR_HEIGHT: f32 = 22.0;

pub(crate) fn centered_text_top(frame_height: f32) -> f32 {
    (frame_height - INPUT_EDITOR_HEIGHT) * 0.5
}

pub(crate) fn input_border(focused: bool) -> Color {
    if focused {
        theme::TEXT_PRIMARY.with_alpha(0.92)
    } else {
        INPUT_BORDER
    }
}

/// Read committed text, excluding IME preedit text (which is not a user submission).
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
        assert_eq!(input_border(true), theme::TEXT_PRIMARY.with_alpha(0.92));
    }

    #[test]
    fn line_box_is_centered_within_search_field() {
        assert_eq!(centered_text_top(40.0), 9.0);
    }
}
