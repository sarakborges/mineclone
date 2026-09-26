use bevy::prelude::Vec3;

use super::game_mode::GameMode;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PlayerSaveData {
    position: Option<Vec3>,
    game_mode: GameMode,
    health: Option<f32>,
    look: Option<(f32, f32)>,
    flying: bool,
}

impl PlayerSaveData {
    pub(crate) fn position(&self) -> Option<Vec3> {
        self.position
    }

    pub(crate) fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    pub(crate) fn health(&self) -> Option<f32> {
        self.health
    }

    pub(crate) fn look(&self) -> Option<(f32, f32)> {
        self.look
    }

    pub(crate) fn flying(&self) -> bool {
        self.flying
    }

    pub(crate) fn save_with_health(
        &mut self,
        position: Vec3,
        game_mode: GameMode,
        health: Option<f32>,
        look: Option<(f32, f32)>,
        flying: bool,
    ) {
        self.position = Some(position);
        self.game_mode = game_mode;
        self.health = health;
        self.look = look;
        self.flying = flying;
    }
}
