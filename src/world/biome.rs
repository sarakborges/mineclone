use bevy::prelude::*;

pub const DEFAULT_BIOME_ID: &str = "mineclone:overworld/plains";

#[derive(Resource)]
pub struct CurrentBiome {
    pub id: String,
}

impl Default for CurrentBiome {
    fn default() -> Self {
        Self {
            id: DEFAULT_BIOME_ID.to_owned(),
        }
    }
}
