use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum GameMode {
    Creative,
    Spectator,
    #[default]
    Survival,
}

impl GameMode {
    pub(crate) const fn has_creative_inventory(self) -> bool {
        matches!(self, Self::Creative)
    }

    pub(crate) const fn allows_flight(self) -> bool {
        matches!(self, Self::Creative | Self::Spectator)
    }

    pub(crate) const fn is_spectator(self) -> bool {
        matches!(self, Self::Spectator)
    }
}

pub(crate) fn not_spectator(game_mode: Single<&GameMode>) -> bool {
    !game_mode.is_spectator()
}
