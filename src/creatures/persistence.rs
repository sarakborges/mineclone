use std::io;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::creature::CreatureRegistry;

use super::EntityMetaTags;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedCreature {
    pub(crate) definition_id: String,
    pub(crate) position: [f32; 3],
    pub(crate) health: f32,
    #[serde(default)]
    pub(crate) meta_tags: EntityMetaTags,
}

impl SavedCreature {
    pub(crate) fn validate(&self, definitions: &CreatureRegistry) -> io::Result<()> {
        if definitions.get(&self.definition_id).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown saved creature definition: {}", self.definition_id),
            ));
        }
        if self.position.iter().any(|value| !value.is_finite())
            || !self.health.is_finite()
            || self.health <= 0.0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "saved creature position or health is invalid",
            ));
        }
        self.meta_tags
            .validate()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(())
    }
}

#[derive(Resource, Default)]
pub(crate) struct PendingCreatureRestores {
    creatures: Vec<SavedCreature>,
}

impl PendingCreatureRestores {
    pub(crate) fn new(creatures: Vec<SavedCreature>) -> Self {
        Self { creatures }
    }

    pub(crate) fn saved(&self) -> &[SavedCreature] {
        &self.creatures
    }

    pub(super) fn is_empty(&self) -> bool {
        self.creatures.is_empty()
    }

    pub(super) fn take(&mut self) -> Vec<SavedCreature> {
        std::mem::take(&mut self.creatures)
    }

    pub(super) fn defer(&mut self, creature: SavedCreature) {
        self.creatures.push(creature);
    }
}
