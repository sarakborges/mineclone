use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    creatures::SavedCreature,
    gameplay::storage_box::StorageBoxStorage,
    voxel::world::VoxelWorld,
};

use super::fluid_updates::SavedFluidUpdates;

pub(crate) struct InactiveDimensionState {
    world: VoxelWorld,
    storage_boxes: StorageBoxStorage,
    fluid_updates: SavedFluidUpdates,
    creatures: Vec<SavedCreature>,
}

impl Default for InactiveDimensionState {
    fn default() -> Self {
        Self {
            world: VoxelWorld::default(),
            storage_boxes: StorageBoxStorage::default(),
            fluid_updates: SavedFluidUpdates::default(),
            creatures: Vec::new(),
        }
    }
}

impl InactiveDimensionState {
    pub(crate) fn new(
        world: VoxelWorld,
        storage_boxes: StorageBoxStorage,
        fluid_updates: SavedFluidUpdates,
        creatures: Vec<SavedCreature>,
    ) -> Self {
        Self {
            world,
            storage_boxes,
            fluid_updates,
            creatures,
        }
    }

    pub(crate) fn world(&self) -> &VoxelWorld {
        &self.world
    }

    pub(crate) fn storage_boxes(&self) -> &StorageBoxStorage {
        &self.storage_boxes
    }

    pub(crate) fn fluid_updates(&self) -> &SavedFluidUpdates {
        &self.fluid_updates
    }

    pub(crate) fn creatures(&self) -> &[SavedCreature] {
        &self.creatures
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        VoxelWorld,
        StorageBoxStorage,
        SavedFluidUpdates,
        Vec<SavedCreature>,
    ) {
        (
            self.world,
            self.storage_boxes,
            self.fluid_updates,
            self.creatures,
        )
    }
}

#[derive(Resource, Default)]
pub(crate) struct InactiveDimensionStates {
    states: HashMap<String, InactiveDimensionState>,
}

impl InactiveDimensionStates {
    pub(crate) fn insert(&mut self, dimension_id: String, state: InactiveDimensionState) {
        let previous = self.states.insert(dimension_id, state);
        assert!(
            previous.is_none(),
            "inactive dimension state must not be replaced without activation"
        );
    }

    pub(crate) fn take(&mut self, dimension_id: &str) -> Option<InactiveDimensionState> {
        self.states.remove(dimension_id)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &InactiveDimensionState)> {
        self.states.iter().map(|(id, state)| (id.as_str(), state))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}
