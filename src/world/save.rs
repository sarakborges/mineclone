use std::collections::HashMap;

use bevy::prelude::*;

use crate::player::{game_mode::GameMode, player_id::PlayerId, save::PlayerSaveData};

use super::seed::WorldSeed;

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

    pub fn player_position(&self, player_id: PlayerId) -> Option<Vec3> {
        self.players.get(&player_id).and_then(PlayerSaveData::position)
    }

    pub fn player_game_mode(&self, player_id: PlayerId) -> GameMode {
        self.players
            .get(&player_id)
            .map(PlayerSaveData::game_mode)
            .unwrap_or_default()
    }

    pub fn begin_new_world(&mut self, seed: WorldSeed, dimension_id: &str) {
        self.seed = Some(seed.0);
        self.dimension_id = Some(dimension_id.to_owned());
        self.players.clear();
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
