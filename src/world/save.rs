use bevy::prelude::*;

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
    player_position: Option<Vec3>,
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

    pub fn player_position(&self) -> Option<Vec3> {
        self.player_position
    }

    pub fn begin_new_world(&mut self, seed: WorldSeed, dimension_id: &str) {
        self.seed = Some(seed.0);
        self.dimension_id = Some(dimension_id.to_owned());
        self.player_position = None;
    }

    pub fn save_player_position(&mut self, position: Vec3) {
        if self.has_world() {
            self.player_position = Some(position);
        }
    }
}
