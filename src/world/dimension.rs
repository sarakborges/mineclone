use bevy::prelude::*;

pub const DEFAULT_DIMENSION_ID: &str = "mineclone:overworld";
pub const DEFAULT_DIMENSION_NAME: &str = "Overworld";

#[derive(Resource)]
pub struct CurrentDimension {
    pub id: &'static str,
    pub name: &'static str,
}

impl Default for CurrentDimension {
    fn default() -> Self {
        Self {
            id: DEFAULT_DIMENSION_ID,
            name: DEFAULT_DIMENSION_NAME,
        }
    }
}
