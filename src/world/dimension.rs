use bevy::prelude::*;

pub const DEFAULT_DIMENSION_ID: &str = "mineclone:overworld";

#[derive(Resource)]
pub struct CurrentDimension {
    pub id: String,
}

impl Default for CurrentDimension {
    fn default() -> Self {
        Self {
            id: DEFAULT_DIMENSION_ID.to_owned(),
        }
    }
}
