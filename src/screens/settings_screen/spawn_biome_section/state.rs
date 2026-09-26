use crate::ui::dropdown::DropdownState;

pub(in crate::screens::settings_screen) struct SpawnBiomeDropdownKind;
pub(in crate::screens::settings_screen) type SpawnBiomeDropdownState =
    DropdownState<SpawnBiomeDropdownKind>;
