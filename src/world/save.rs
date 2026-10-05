use std::collections::HashMap;

use bevy::prelude::*;

use crate::player::{game_mode::GameMode, player_id::PlayerId, save::PlayerSaveData};

use super::{game_rules::GameRules, seed::WorldSeed};

#[derive(Debug, Resource, Clone, Copy, Default, PartialEq, Eq)]
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

    pub(crate) fn prepare_dimension_warp(&mut self, dimension_id: &str) {
        assert!(self.has_world(), "dimension warp requires an active world");
        self.dimension_id = Some(dimension_id.to_owned());
    }

    pub(crate) fn player(&self, player_id: PlayerId) -> Option<PlayerSaveData> {
        self.players.get(&player_id).copied()
    }

    pub(crate) fn save_game_rules(&mut self, game_rules: GameRules) {
        if self.has_world() {
            self.game_rules = game_rules;
        }
    }

    pub(crate) fn save_player_state_with_health(
        &mut self,
        player_id: PlayerId,
        position: Vec3,
        game_mode: GameMode,
        health: Option<f32>,
        look: Option<(f32, f32)>,
        flying: bool,
    ) {
        if self.has_world() {
            self.players
                .entry(player_id)
                .or_default()
                .save_with_health(position, game_mode, health, look, flying);
        }
    }
}
