use bevy::text::EditableText;

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
}
