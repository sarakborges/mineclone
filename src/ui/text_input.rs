use bevy::prelude::*;

#[derive(Clone, Default)]
pub(crate) struct TextInputState {
    text: String,
    focused: bool,
    replace_on_next_input: bool,
}

impl TextInputState {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn focused(&self) -> bool {
        self.focused
    }

    pub(crate) fn focus(&mut self) {
        self.focused = true;
        self.replace_on_next_input = false;
    }

    pub(crate) fn blur(&mut self) {
        self.focused = false;
        self.replace_on_next_input = false;
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn select_all(&mut self) {
        if self.focused {
            self.replace_on_next_input = true;
        }
    }

    pub(crate) fn push_text(&mut self, text: &str) -> bool {
        let filtered = text
            .chars()
            .filter(|character| !character.is_control())
            .collect::<String>();
        if filtered.is_empty() {
            return false;
        }

        if self.replace_on_next_input {
            self.text.clear();
            self.replace_on_next_input = false;
        }

        self.text.push_str(&filtered);
        true
    }

    pub(crate) fn backspace(&mut self) -> bool {
        if self.replace_on_next_input {
            let changed = !self.text.is_empty();
            self.text.clear();
            self.replace_on_next_input = false;
            return changed;
        }

        self.text.pop().is_some()
    }
}

pub(crate) fn select_all_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    let control_pressed =
        keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let control_just_pressed =
        keys.just_pressed(KeyCode::ControlLeft) || keys.just_pressed(KeyCode::ControlRight);

    (control_pressed && keys.just_pressed(KeyCode::KeyA))
        || (keys.pressed(KeyCode::KeyA) && control_just_pressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_select_all_replaces_on_next_text() {
        let mut input = TextInputState::default();
        input.focus();
        assert!(input.push_text("forest"));
        input.select_all();
        assert!(input.push_text("plains"));

        assert_eq!(input.text(), "plains");
    }

    #[test]
    fn text_input_filters_control_characters() {
        let mut input = TextInputState::default();
        input.focus();
        assert!(input.push_text("witch\nwood"));

        assert_eq!(input.text(), "witchwood");
    }

    #[test]
    fn text_input_backspace_clears_selected_text() {
        let mut input = TextInputState::default();
        input.focus();
        assert!(input.push_text("mountains"));
        input.select_all();
        assert!(input.backspace());

        assert!(input.text().is_empty());
    }
}
