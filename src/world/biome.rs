use bevy::prelude::*;

pub const DEFAULT_BIOME_ID: &str = "mineclone:overworld/plains";
pub const DEFAULT_BIOME_NAME: &str = "Plains";

#[derive(Resource)]
pub struct CurrentBiome {
    pub id: &'static str,
    pub name: &'static str,
}

impl Default for CurrentBiome {
    fn default() -> Self {
        Self {
            id: DEFAULT_BIOME_ID,
            name: DEFAULT_BIOME_NAME,
        }
    }
}
