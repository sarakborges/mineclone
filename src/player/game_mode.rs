use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum GameMode {
    #[default]
    Creative,
}
