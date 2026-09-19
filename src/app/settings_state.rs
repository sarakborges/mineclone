use bevy::prelude::*;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum SettingsState {
    #[default]
    Closed,
    Open,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsScreenMode {
    #[default]
    Game,
    World,
}
