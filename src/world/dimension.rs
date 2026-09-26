use std::collections::HashMap;

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

#[derive(Resource, Default)]
pub(crate) struct DimensionEntityCounts {
    pub(crate) total: usize,
    pub(crate) entities: HashMap<String, usize>,
}

impl DimensionEntityCounts {
    pub(crate) fn rebuild<'a>(&mut self, creatures: impl Iterator<Item = &'a crate::creatures::CreatureInstance>) {
        self.total = 0;
        self.entities.clear();
        for creature in creatures {
            self.total += 1;
            *self.entities.entry(creature.definition_id.clone()).or_default() += 1;
        }
    }

    pub(crate) fn count(&self, entity_id: &str) -> usize {
        self.entities.get(entity_id).copied().unwrap_or(0)
    }
}
