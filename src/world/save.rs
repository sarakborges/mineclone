use std::collections::HashMap;

use bevy::prelude::*;

use crate::player::{game_mode::GameMode, player_id::PlayerId, save::PlayerSaveData};

use super::{DEFAULT_BIOME_SIZE_MULTIPLIER, game_rules::GameRules, seed::WorldSeed};

#[derive(Resource, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorldLoadMode {
    #[default]
    New,
    Load,
}

#[derive(Resource, Default)]
pub struct InMemoryWorldSave {
    seed: Option<u64>,
    dimension_id: Option<String>,
    spawn_biome: Option<String>,
    biome_size_multiplier: Option<f32>,
    game_rules: GameRules,
    players: HashMap<PlayerId, PlayerSaveData>,
}

impl InMemoryWorldSave {
    pub fn has_world(&self) -> bool {
        self.seed.is_some() && self.dimension_id.is_some()
    }

    pub(crate) fn game_rules(&self) -> GameRules {
        self.game_rules
    }

    pub(crate) fn spawn_biome(&self) -> Option<&str> {
        self.spawn_biome.as_deref()
    }

    pub(crate) fn biome_size_multiplier(&self) -> f32 {
        self.biome_size_multiplier
            .unwrap_or(DEFAULT_BIOME_SIZE_MULTIPLIER)
    }

    pub fn player_position(&self, player_id: PlayerId) -> Option<Vec3> {
        self.players.get(&player_id).and_then(PlayerSaveData::position)
    }

    pub fn player_game_mode(&self, player_id: PlayerId) -> GameMode {
        self.players
            .get(&player_id)
            .map(PlayerSaveData::game_mode)
            .unwrap_or_default()
    }

    pub(crate) fn player_health(&self, player_id: PlayerId) -> Option<f32> {
        self.players.get(&player_id).and_then(PlayerSaveData::health)
    }

    pub fn begin_new_world(
        &mut self,
        seed: WorldSeed,
        dimension_id: &str,
        game_rules: GameRules,
        spawn_biome: Option<&str>,
        biome_size_multiplier: f32,
    ) {
        self.seed = Some(seed.0);
        self.dimension_id = Some(dimension_id.to_owned());
        self.spawn_biome = spawn_biome.map(str::to_owned);
        self.biome_size_multiplier = Some(biome_size_multiplier);
        self.game_rules = game_rules;
        self.players.clear();
    }

    pub(crate) fn save_game_rules(&mut self, game_rules: GameRules) {
        if self.has_world() {
            self.game_rules = game_rules;
        }
    }

    pub fn save_player_state(
        &mut self,
        player_id: PlayerId,
        position: Vec3,
        game_mode: GameMode,
    ) {
        if self.has_world() {
            self.players
                .entry(player_id)
                .or_default()
                .save(position, game_mode);
        }
    }

    pub(crate) fn save_player_state_with_health(
        &mut self,
        player_id: PlayerId,
        position: Vec3,
        game_mode: GameMode,
        health: Option<f32>,
    ) {
        if self.has_world() {
            self.players
                .entry(player_id)
                .or_default()
                .save_with_health(position, game_mode, health);
        }
    }
}
