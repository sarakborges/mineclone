use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PlayerId(pub(crate) u64);

pub(crate) const LOCAL_PLAYER_ID: PlayerId = PlayerId(1);
