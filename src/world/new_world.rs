use bevy::prelude::*;

use crate::player::game_mode::GameMode;

use super::{game_rules::GameRules, WorldSeed};

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct NewWorldConfig {
    seed: WorldSeed,
    game_mode: GameMode,
    game_rules: GameRules,
}

impl Default for NewWorldConfig {
    fn default() -> Self {
        Self {
            seed: WorldSeed::fresh(),
            game_mode: GameMode::default(),
            game_rules: GameRules::default(),
        }
    }
}

impl NewWorldConfig {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn seed(&self) -> WorldSeed {
        self.seed
    }

    pub(crate) fn set_seed(&mut self, seed: u64) {
        self.seed = WorldSeed(seed);
    }

    pub(crate) fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    pub(crate) fn set_game_mode(&mut self, game_mode: GameMode) {
        self.game_mode = game_mode;
    }

    pub(crate) fn game_rules(&self) -> GameRules {
        self.game_rules
    }

    pub(crate) fn set_ticks_per_second(&mut self, ticks_per_second: u32) {
        self.game_rules.set_ticks_per_second(ticks_per_second);
    }
}
