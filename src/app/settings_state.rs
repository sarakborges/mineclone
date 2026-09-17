use bevy::prelude::*;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum SettingsState {
    #[default]
    Closed,
    Open,
}

/// Controls which settings are available when the pause menu opens settings.
/// World creation retains its own independent settings context.
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Resource)]
pub enum SettingsScope {
    #[default]
    Game,
    World,
}
