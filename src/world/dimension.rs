use bevy::prelude::*;

use crate::content::builtin_ids::OVERWORLD_DIMENSION_ID;

pub const DEFAULT_DIMENSION_ID: &str = OVERWORLD_DIMENSION_ID;

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
