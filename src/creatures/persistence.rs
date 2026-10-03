use std::io;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{content::creature::CreatureRegistry, entity::EntityHealth};

use super::{CreatureInstance, EntityMetaTags};

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
    pub(crate) fn from_runtime(
        instance: &CreatureInstance,
        transform: &Transform,
        health: &EntityHealth,
        meta_tags: &EntityMetaTags,
    ) -> Option<Self> {
        (!health.is_dead()).then(|| Self {
            definition_id: instance.definition_id.clone(),
            position: transform.translation.to_array(),
            health: health.current(),
            meta_tags: meta_tags.clone(),
        })
    }

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

pub(crate) fn sort_saved_creatures(creatures: &mut [SavedCreature]) {
    creatures.sort_unstable_by(|left, right| {
        left.definition_id
            .cmp(&right.definition_id)
            .then_with(|| left.position[0].total_cmp(&right.position[0]))
            .then_with(|| left.position[1].total_cmp(&right.position[1]))
            .then_with(|| left.position[2].total_cmp(&right.position[2]))
    });
}

#[derive(Resource, Default)]
pub(crate) struct PendingCreatureRestores {
    creatures: Vec<SavedCreature>,
}

impl PendingCreatureRestores {
    pub(crate) fn new(creatures: Vec<SavedCreature>) -> Self {
        Self { creatures }
    }

    pub(crate) fn snapshot(
        &self,
        active: impl Iterator<Item = SavedCreature>,
    ) -> Vec<SavedCreature> {
        let mut creatures = self.creatures.clone();
        creatures.extend(active);
        sort_saved_creatures(&mut creatures);
        creatures
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
