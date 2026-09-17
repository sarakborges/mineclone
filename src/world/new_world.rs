use bevy::prelude::*;

use crate::player::game_mode::GameMode;

use super::{WorldSeed, game_rules::GameRules, world_names::DEFAULT_WORLD_NAME};

#[derive(Resource, Debug, Clone)]
pub(crate) struct NewWorldConfig {
    name: String,
    seed: WorldSeed,
    game_mode: GameMode,
    game_rules: GameRules,
    spawn_biome: Option<String>,
}

impl Default for NewWorldConfig {
    fn default() -> Self {
        Self {
            name: DEFAULT_WORLD_NAME.to_owned(),
            seed: WorldSeed::default(),
            game_mode: GameMode::default(),
            game_rules: GameRules::default(),
            spawn_biome: None,
        }
    }
}

impl NewWorldConfig {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn set_name(&mut self, name: String) {
        if self.name != name {
            self.name = name;
        }
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

    pub(crate) fn spawn_biome(&self) -> Option<&str> {
        self.spawn_biome.as_deref()
    }

    pub(crate) fn set_spawn_biome(&mut self, spawn_biome: Option<String>) {
        if self.spawn_biome != spawn_biome {
            self.spawn_biome = spawn_biome;
        }
    }
}
