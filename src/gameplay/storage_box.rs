use std::{collections::HashMap, io};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{
        block::BlockRegistry,
        block_id::intern_block_id,
        item::ItemRegistry,
        item_id::intern_item_id,
        layer::LayerRegistry,
        layer_id::intern_layer_id,
        object::ObjectRegistry,
        object_id::intern_object_id,
        tool::ToolRegistry,
        tool_id::intern_tool_id,
    },
    gameplay::{availability::world_interaction_available, modal::GameplayModalState},
    player::{camera::GameplayCamera, game_mode::GameMode, item_stack::{ItemStack, SavedItemStack}},
    targeting::block::{BlockTargetingSet, TargetedBlock},
    voxel::{coordinates::chunk_coord_from_world, world::VoxelWorld},
    world_items::WorldItemSpawnRequest,
};

pub(crate) const STORAGE_BOX_BLOCK_ID: &str = "asteria:storage_box";
pub(crate) const STORAGE_BOX_SLOT_COUNT: usize = 27;

#[derive(Clone, Debug)]
pub(crate) struct StorageBoxInventory {
    slots: [Option<ItemStack>; STORAGE_BOX_SLOT_COUNT],
}

impl Default for StorageBoxInventory {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }
}

impl StorageBoxInventory {
    pub(crate) fn stack_at(&self, index: usize) -> Option<&ItemStack> {
        self.slots.get(index).and_then(Option::as_ref)
    }

    pub(crate) fn slot_mut(&mut self, index: usize) -> Option<&mut Option<ItemStack>> {
        self.slots.get_mut(index)
    }

    fn try_insert_stack(&mut self, stack: ItemStack) -> Result<(), ItemStack> {
        let mut remaining = Some(stack);
        for existing in self.slots.iter_mut().flatten() {
            let Some(incoming) = remaining.as_ref() else {
                return Ok(());
            };
            if !existing.can_stack_with(incoming) {
                continue;
            }
            let incoming = remaining
                .take()
                .expect("storage remainder must exist after compatibility check");
            remaining = existing.merge_from(incoming);
        }

        let Some(stack) = remaining else {
            return Ok(());
        };
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(stack);
            Ok(())
        } else {
            Err(stack)
        }
    }

    fn saved_items(&self) -> Vec<Option<SavedItemStack>> {
        self.slots
            .iter()
            .map(|stack| stack.as_ref().map(ItemStack::saved))
            .collect()
    }

    fn into_stacks(self) -> Vec<ItemStack> {
        self.slots.into_iter().flatten().collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedStorageBox {
    position: [i32; 3],
    items: Vec<Option<SavedItemStack>>,
}

impl SavedStorageBox {
    pub(crate) fn position(&self) -> [i32; 3] {
        self.position
    }

    pub(crate) fn items(&self) -> &[Option<SavedItemStack>] {
        &self.items
    }
}

#[derive(Resource, Default)]
pub(crate) struct StorageBoxStorage {
    boxes: HashMap<IVec3, StorageBoxInventory>,
    active: Option<IVec3>,
}

impl StorageBoxStorage {
    pub(crate) fn open(&mut self, position: IVec3) {
        self.boxes.entry(position).or_default();
        self.active = Some(position);
    }

    pub(crate) fn close(&mut self) {
        self.active = None;
    }

    pub(crate) fn active_position(&self) -> Option<IVec3> {
        self.active
    }

    pub(crate) fn active_stack_at(&self, index: usize) -> Option<&ItemStack> {
        self.active
            .and_then(|position| self.boxes.get(&position))
            .and_then(|inventory| inventory.stack_at(index))
    }

    pub(crate) fn active_slot_mut(&mut self, index: usize) -> Option<&mut Option<ItemStack>> {
        let position = self.active?;
        self.boxes.get_mut(&position)?.slot_mut(index)
    }

    pub(crate) fn try_insert_active(&mut self, stack: ItemStack) -> Result<(), ItemStack> {
        let Some(position) = self.active else {
            return Err(stack);
        };
        self.boxes
            .entry(position)
            .or_default()
            .try_insert_stack(stack)
    }

    pub(crate) fn drain(&mut self, position: IVec3) -> Vec<ItemStack> {
        if self.active == Some(position) {
            self.active = None;
        }
        self.boxes
            .remove(&position)
            .map(StorageBoxInventory::into_stacks)
            .unwrap_or_default()
    }

    pub(crate) fn saved_boxes(&self) -> Vec<SavedStorageBox> {
        let mut boxes = self
            .boxes
            .iter()
            .filter(|(_, inventory)| inventory.slots.iter().any(Option::is_some))
            .map(|(position, inventory)| SavedStorageBox {
                position: position.to_array(),
                items: inventory.saved_items(),
            })
            .collect::<Vec<_>>();
        boxes.sort_unstable_by_key(|saved| saved.position);
        boxes
    }

    pub(crate) fn from_saved_boxes(
        saved_boxes: &[SavedStorageBox],
        items: &ItemRegistry,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        objects: &ObjectRegistry,
        tools: &ToolRegistry,
    ) -> io::Result<Self> {
        let mut storage = Self::default();
        for saved_box in saved_boxes {
            if saved_box.items.len() != STORAGE_BOX_SLOT_COUNT {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid storage box inventory length",
                ));
            }

            let position = IVec3::from_array(saved_box.position);
            if storage.boxes.contains_key(&position) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "duplicate storage box position",
                ));
            }

            let mut inventory = StorageBoxInventory::default();
            for (index, saved) in saved_box.items.iter().enumerate() {
                let restored = saved
                    .as_ref()
                    .map(|stack| {
                        stack.restore(|id| {
                            if items.get(id).is_some() {
                                Some(intern_item_id(id))
                            } else if blocks.get(id).is_some() {
                                Some(intern_block_id(id))
                            } else if layers.get(id).is_some() {
                                Some(intern_layer_id(id))
                            } else if objects.get(id).is_some() {
                                Some(intern_object_id(id))
                            } else if tools.get(id).is_some() {
                                Some(intern_tool_id(id))
                            } else {
                                None
                            }
                        })
                    })
                    .transpose()?;
                inventory.slots[index] = restored;
            }
            storage.boxes.insert(position, inventory);
        }
        Ok(storage)
    }

    fn occupied_positions(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.boxes.keys().copied()
    }
}

#[derive(Resource, Default)]
pub(crate) struct StorageInteractionConsumed(bool);

impl StorageInteractionConsumed {
    pub(crate) fn is_consumed(&self) -> bool {
        self.0
    }
}

pub(super) struct StorageBoxPlugin;

impl Plugin for StorageBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StorageBoxStorage>()
            .init_resource::<StorageInteractionConsumed>()
            .add_systems(
                OnEnter(GameState::NewWorld),
                reset_resource::<StorageBoxStorage>,
            )
            .add_systems(
                OnEnter(GameState::StartingScreen),
                reset_resource::<StorageBoxStorage>,
            )
            .add_systems(
                Update,
                open_targeted_storage_box
                    .after(BlockTargetingSet::Raycast)
                    .before(BlockTargetingSet::Interaction)
                    .run_if(world_interaction_available),
            )
            .add_systems(
                Update,
                cleanup_removed_storage_boxes
                    .after(BlockTargetingSet::Interaction)
                    .before(BlockTargetingSet::Visuals)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(Last, reset_interaction_consumed);
    }
}

fn open_targeted_storage_box(
    buttons: Res<ButtonInput<MouseButton>>,
    game_mode: Single<&GameMode, With<GameplayCamera>>,
    targeted: Res<TargetedBlock>,
    mut storage: ResMut<StorageBoxStorage>,
    mut consumed: ResMut<StorageInteractionConsumed>,
    mut modal: ResMut<NextState<GameplayModalState>>,
) {
    if !buttons.just_pressed(MouseButton::Right) || game_mode.is_spectator() {
        return;
    }
    let Some(hit) = targeted.0 else {
        return;
    };
    if hit.block_id != STORAGE_BOX_BLOCK_ID {
        return;
    }

    storage.open(hit.voxel);
    consumed.0 = true;
    modal.set(GameplayModalState::StorageBox);
}

fn cleanup_removed_storage_boxes(
    world: Res<VoxelWorld>,
    mut storage: ResMut<StorageBoxStorage>,
    mut drops: MessageWriter<WorldItemSpawnRequest>,
) {
    let removed = storage
        .occupied_positions()
        .filter(|position| world.chunk(chunk_coord_from_world(*position)).is_some())
        .filter(|position| {
            world
                .cell_at(*position)
                .is_none_or(|cell| cell.block_id != STORAGE_BOX_BLOCK_ID)
        })
        .collect::<Vec<_>>();

    for position in removed {
        let drop_position = position.as_vec3() + Vec3::splat(0.5);
        for stack in storage.drain(position) {
            drops.write(WorldItemSpawnRequest::dropped(stack, drop_position));
        }
    }
}

fn reset_interaction_consumed(mut consumed: ResMut<StorageInteractionConsumed>) {
    consumed.0 = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_box_has_exactly_twenty_seven_slots() {
        let inventory = StorageBoxInventory::default();
        assert_eq!(inventory.slots.len(), 27);
    }

    #[test]
    fn saved_empty_storage_is_omitted() {
        let mut storage = StorageBoxStorage::default();
        storage.open(IVec3::new(1, 2, 3));
        assert!(storage.saved_boxes().is_empty());
    }
}
