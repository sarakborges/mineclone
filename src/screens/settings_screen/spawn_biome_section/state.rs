use bevy::prelude::*;

#[derive(Resource, Default)]
pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownState {
    pub(super) open: bool,
}

impl SpawnBiomeDropdownState {
    pub(in crate::screens::settings_screen) fn input_editing(&self) -> bool {
        self.open
    }

    pub(in crate::screens::settings_screen) fn reset(&mut self) {
        self.open = false;
    }

    pub(in crate::screens::settings_screen) fn open(&mut self) {
        self.open = true;
    }

    pub(in crate::screens::settings_screen) fn close(&mut self) {
        self.open = false;
    }
}
