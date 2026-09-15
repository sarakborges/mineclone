use bevy::prelude::*;

use crate::ui::text_input::TextInputState;

#[derive(Resource, Default)]
pub(super) struct SpawnBiomeDropdownState {
    pub(super) open: bool,
    pub(super) search: TextInputState,
}

impl SpawnBiomeDropdownState {
    pub(super) fn input_editing(&self) -> bool {
        self.open || self.search.focused()
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(super) fn open(&mut self) {
        self.open = true;
        self.search.reset();
        self.search.focus();
    }

    pub(super) fn close(&mut self) {
        self.open = false;
        self.search.reset();
    }
}
