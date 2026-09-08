use bevy::prelude::*;

pub const DEFAULT_DIMENSION_ID: &str = "mineclone:overworld";

#[derive(Resource)]
pub struct CurrentDimension {
    pub id: &'static str,
}

impl Default for CurrentDimension {
    fn default() -> Self {
        Self {
            id: DEFAULT_DIMENSION_ID,
        }
    }
}
