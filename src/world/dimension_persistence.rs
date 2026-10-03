use std::{collections::HashMap, io};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::fluid::FluidRegistry,
    creatures::{CreatureInstance, EntityMetaTags, PendingCreatureRestores, SavedCreature},
    entity::EntityHealth,
    gameplay::storage_box::StorageBoxStorage,
    voxel::world::VoxelWorld,
};

use super::{
    dimension::DimensionId,
    fluid_updates::{PendingFluidUpdates, SavedFluidUpdates},
    tick::WorldTickClock,
};

pub(crate) struct InactiveDimensionState {
    world: VoxelWorld,
    spawn_biome: Option<String>,
    storage_boxes: StorageBoxStorage,
    fluid_updates: SavedFluidUpdates,
    creatures: Vec<SavedCreature>,
}

impl InactiveDimensionState {
    pub(crate) fn new(
        world: VoxelWorld,
        spawn_biome: Option<String>,
        storage_boxes: StorageBoxStorage,
        fluid_updates: SavedFluidUpdates,
        creatures: Vec<SavedCreature>,
    ) -> Self {
        Self {
            world,
            spawn_biome,
            storage_boxes,
            fluid_updates,
            creatures,
        }
    }

    fn empty(spawn_biome: Option<String>) -> Self {
        Self::new(
            VoxelWorld::default(),
            spawn_biome,
            StorageBoxStorage::default(),
            SavedFluidUpdates::default(),
            Vec::new(),
        )
    }

    pub(crate) fn world(&self) -> &VoxelWorld {
        &self.world
    }

    pub(crate) fn spawn_biome(&self) -> Option<&str> {
        self.spawn_biome.as_deref()
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

    fn into_parts(
        self,
    ) -> (
        VoxelWorld,
        Option<String>,
        StorageBoxStorage,
        SavedFluidUpdates,
        Vec<SavedCreature>,
    ) {
        (
            self.world,
            self.spawn_biome,
            self.storage_boxes,
            self.fluid_updates,
            self.creatures,
        )
    }
}

#[derive(Resource, Default)]
pub(crate) struct InactiveDimensionStates {
    states: HashMap<DimensionId, InactiveDimensionState>,
}

impl InactiveDimensionStates {
    pub(crate) fn insert(&mut self, dimension_id: DimensionId, state: InactiveDimensionState) {
        let previous = self.states.insert(dimension_id, state);
        assert!(
            previous.is_none(),
            "inactive dimension state must not be replaced without activation"
        );
    }

    pub(crate) fn get(&self, dimension_id: &DimensionId) -> Option<&InactiveDimensionState> {
        self.states.get(dimension_id)
    }

    pub(crate) fn take(&mut self, dimension_id: &DimensionId) -> Option<InactiveDimensionState> {
        self.states.remove(dimension_id)
    }

    pub(crate) fn iter(
        &self,
    ) -> impl Iterator<Item = (&DimensionId, &InactiveDimensionState)> {
        self.states.iter()
    }
}

type DimensionCreatureQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static CreatureInstance,
        &'static Transform,
        &'static EntityHealth,
        &'static EntityMetaTags,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct DimensionRuntimeContext<'w, 's> {
    world: ResMut<'w, VoxelWorld>,
    storage_boxes: ResMut<'w, StorageBoxStorage>,
    pending_fluids: ResMut<'w, PendingFluidUpdates>,
    pending_creatures: ResMut<'w, PendingCreatureRestores>,
    inactive_dimensions: ResMut<'w, InactiveDimensionStates>,
    fluids: Res<'w, FluidRegistry>,
    world_tick: Res<'w, WorldTickClock>,
    creatures: DimensionCreatureQuery<'w, 's>,
}

impl DimensionRuntimeContext<'_, '_> {
    pub(crate) fn world(&self) -> &VoxelWorld {
        &self.world
    }

    pub(crate) fn inactive_spawn_biome(
        &self,
        dimension_id: &DimensionId,
    ) -> Option<Option<&str>> {
        self.inactive_dimensions
            .get(dimension_id)
            .map(InactiveDimensionState::spawn_biome)
    }

    pub(crate) fn swap_to(
        &mut self,
        current_dimension: &DimensionId,
        current_spawn_biome: Option<String>,
        target_dimension: DimensionId,
        new_target_spawn_biome: Option<String>,
    ) -> io::Result<Option<String>> {
        let current_fluid_updates = self
            .pending_fluids
            .capture_saved(self.world_tick.current_tick(), &self.fluids)?;
        let current_creatures = self.pending_creatures.snapshot(
            self.creatures
                .iter()
                .filter_map(|(instance, transform, health, meta_tags)| {
                    SavedCreature::from_runtime(instance, transform, health, meta_tags)
                }),
        );

        let target_pending_fluids = self
            .inactive_dimensions
            .get(&target_dimension)
            .map(|state| PendingFluidUpdates::from_saved(state.fluid_updates(), &self.fluids))
            .transpose()?
            .unwrap_or_default();
        let target_state = self
            .inactive_dimensions
            .take(&target_dimension)
            .unwrap_or_else(|| InactiveDimensionState::empty(new_target_spawn_biome));

        self.storage_boxes.close();
        let current_state = InactiveDimensionState::new(
            std::mem::take(&mut *self.world),
            current_spawn_biome,
            std::mem::take(&mut *self.storage_boxes),
            current_fluid_updates,
            current_creatures,
        );
        self.inactive_dimensions
            .insert(current_dimension.clone(), current_state);

        let (world, spawn_biome, storage_boxes, _fluid_updates, creatures) =
            target_state.into_parts();
        *self.world = world;
        *self.storage_boxes = storage_boxes;
        *self.pending_fluids = target_pending_fluids;
        *self.pending_creatures = PendingCreatureRestores::new(creatures);

        Ok(spawn_biome)
    }
}
