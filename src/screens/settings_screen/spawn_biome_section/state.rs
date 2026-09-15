use bevy::prelude::*;

use crate::ui::text_input::TextInputState;

#[derive(Resource, Default)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownState {
    pub(super) open: bool,
    pub(super) search: TextInputState,
}

impl SpawnBiomeDropdownState {
    pub(in crate::screens::settings_screen) fn input_editing(&self) -> bool {
        self.open || self.search.focused()
    }

    pub(in crate::screens::settings_screen) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(in crate::screens::settings_screen) fn open(&mut self) {
        self.open = true;
        self.search.reset();
        self.search.focus();
    }

    pub(in crate::screens::settings_screen) fn close(&mut self) {
        self.open = false;
        self.search.reset();
    }
}
