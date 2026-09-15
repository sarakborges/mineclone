use std::collections::HashMap;

use bevy::prelude::*;

use crate::player::{game_mode::GameMode, player_id::PlayerId, save::PlayerSaveData};

use super::{game_rules::GameRules, seed::WorldSeed};

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
    game_rules: GameRules,
    players: HashMap<PlayerId, PlayerSaveData>,
}

impl InMemoryWorldSave {
    pub fn has_world(&self) -> bool {
        self.seed.is_some() && self.dimension_id.is_some()
    }

    pub fn seed(&self) -> Option<WorldSeed> {
        self.seed.map(WorldSeed)
    }

    pub fn dimension_id(&self) -> Option<&str> {
        self.dimension_id.as_deref()
    }

    pub(crate) fn game_rules(&self) -> GameRules {
        self.game_rules
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

    pub fn begin_new_world(
        &mut self,
        seed: WorldSeed,
        dimension_id: &str,
        game_rules: GameRules,
    ) {
        self.seed = Some(seed.0);
        self.dimension_id = Some(dimension_id.to_owned());
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
}
