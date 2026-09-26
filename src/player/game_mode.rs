use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum GameMode {
    Creative,
    #[default]
    Survival,
}

impl GameMode {
    pub(crate) const fn has_creative_inventory(self) -> bool {
        matches!(self, Self::Creative)
    }

    pub(crate) const fn allows_flight(self) -> bool {
        matches!(self, Self::Creative)
    }
}
