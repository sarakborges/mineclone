use bevy::prelude::Vec3;

use super::game_mode::GameMode;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PlayerSaveData {
    position: Option<Vec3>,
    game_mode: GameMode,
}

impl PlayerSaveData {
    pub(crate) fn position(&self) -> Option<Vec3> {
        self.position
    }

    pub(crate) fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    pub(crate) fn save(&mut self, position: Vec3, game_mode: GameMode) {
        self.position = Some(position);
        self.game_mode = game_mode;
    }
}
