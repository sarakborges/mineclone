use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{
        block::BlockRegistry,
        block_shape::is_stackable_layer,
        object::ObjectRegistry,
    },
    player::item_stack::ItemStack,
    world::tick::WorldTickClock,
    world_items::WorldItemSpawnRequest,
    world_objects::detached_object_drop_request,
};

use super::{
    cell::VoxelCell,
    edit::VoxelTopologyRuntime,
    read::VoxelRead,
    stackable_layer::stackable_layer_count,
};

pub(crate) const BLOCK_GRAVITY_TAG: &str = "gravity";

#[derive(Resource, Default)]
pub(crate) struct PendingBlockGravityUpdates {
    queue: VecDeque<IVec3>,
    queued: HashSet<IVec3>,
}

impl PendingBlockGravityUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.enqueue(position);
        self.enqueue(position + IVec3::Y);
    }

    pub(crate) fn enqueue(&mut self, position: IVec3) {
        if position.y < 0 || !self.queued.insert(position) {
            return;
        }
        self.queue.push_back(position);
    }

    pub(crate) fn take_batch(&mut self) -> Vec<IVec3> {
        let batch_len = self.queue.len();
        let mut batch = Vec::with_capacity(batch_len);
        for _ in 0..batch_len {
            let Some(position) = self.queue.pop_front() else {
                break;
            };
            self.queued.remove(&position);
            batch.push(position);
        }
        batch
    }
}

pub(crate) struct BlockGravityPlugin;

impl Plugin for BlockGravityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingBlockGravityUpdates>()
            .add_systems(
                OnEnter(GameState::Loading),
                reset_resource::<PendingBlockGravityUpdates>,
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                reset_resource::<PendingBlockGravityUpdates>,
            )
            .add_systems(
                PostUpdate,
                process_block_gravity.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn process_block_gravity(
    world_ticks: Res<WorldTickClock>,
    blocks: Res<BlockRegistry>,
    objects: Res<ObjectRegistry>,
    mut runtime: VoxelTopologyRuntime,
    mut item_spawns: MessageWriter<WorldItemSpawnRequest>,
) {
    if world_ticks.ticks_this_frame() == 0 {
        return;
    }

    for position in runtime.take_block_gravity_batch() {
        let Some(cell) = runtime.read().cell_at(position) else {
            continue;
        };
        let Some(definition) = blocks.get(cell.block_id) else {
            continue;
        };
        let below = position - IVec3::Y;

        if is_stackable_layer(definition) {
            if below.y >= 0 && !runtime.read().is_loaded_at(below) {
                runtime.enqueue_block_gravity(position);
                continue;
            }
            if runtime.read().cell_at(below).is_some() {
                continue;
            }

            let Some(mutation) = runtime.set_block_detailed(position, None) else {
                runtime.enqueue_block_gravity(position);
                continue;
            };
            item_spawns.write(WorldItemSpawnRequest::dropped(
                stackable_layer_drop_stack(cell),
                position.as_vec3() + Vec3::splat(0.5),
            ));
            emit_detached_object_drops(
                position,
                mutation.previous_cell,
                mutation.detached_objects,
                &objects,
                &mut item_spawns,
            );
            continue;
        }

        if !definition.tags.iter().any(|tag| tag == BLOCK_GRAVITY_TAG) {
            continue;
        }
        if below.y < 0 {
            continue;
        }
        if !runtime.read().is_loaded_at(below) {
            runtime.enqueue_block_gravity(position);
            continue;
        }
        if runtime.read().cell_at(below).is_some() {
            continue;
        }

        if runtime.set_block(below, Some(cell)).is_none() {
            runtime.enqueue_block_gravity(position);
            continue;
        }

        let Some(mutation) = runtime.set_block_detailed(position, None) else {
            let _ = runtime.set_block(below, None);
            runtime.enqueue_block_gravity(position);
            continue;
        };

        emit_detached_object_drops(
            position,
            mutation.previous_cell,
            mutation.detached_objects,
            &objects,
            &mut item_spawns,
        );
    }
}

fn stackable_layer_drop_stack(cell: VoxelCell) -> ItemStack {
    let quantity = stackable_layer_count(cell).unwrap_or(1) as u32;
    ItemStack::new(cell.block_id).with_quantity(quantity)
}

fn emit_detached_object_drops(
    position: IVec3,
    previous_cell: Option<VoxelCell>,
    detached_objects: super::chunk::ObjectCells,
    objects: &ObjectRegistry,
    item_spawns: &mut MessageWriter<WorldItemSpawnRequest>,
) {
    for object in detached_objects {
        if let Some(drop) = detached_object_drop_request(position, previous_cell, object, objects) {
            item_spawns.write(drop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        content::block_orientation::BlockOrientation,
        voxel::{
            stackable_layer::stackable_layer_mask,
            texture_rotation::TextureRotation,
        },
    };

    #[test]
    fn voxel_edit_wakes_changed_voxel_and_block_above_once() {
        let mut pending = PendingBlockGravityUpdates::default();
        let position = IVec3::new(3, 7, -2);

        pending.enqueue_voxel_edit(position);
        pending.enqueue_voxel_edit(position);

        assert_eq!(pending.take_batch(), vec![position, position + IVec3::Y]);
        assert!(pending.take_batch().is_empty());
    }

    #[test]
    fn negative_world_positions_are_not_queued() {
        let mut pending = PendingBlockGravityUpdates::default();
        pending.enqueue(IVec3::new(0, -1, 0));
        assert!(pending.take_batch().is_empty());
    }

    #[test]
    fn stackable_layer_drop_preserves_layer_count() {
        let cell = stackable_layer_mask(4).apply_to_cell(
            VoxelCell::oriented(
                "asteria:snow_layer",
                TextureRotation::default(),
                BlockOrientation::Y,
            ),
            false,
        );

        let stack = stackable_layer_drop_stack(cell);
        assert_eq!(stack.id(), "asteria:snow_layer");
        assert_eq!(stack.quantity(), 4);
    }
}
