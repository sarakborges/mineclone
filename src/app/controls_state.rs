use bevy::prelude::*;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum ControlsState {
    #[default]
    Closed,
    Open,
}
