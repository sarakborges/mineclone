use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::crash_log::log_gameplay_event,
    content::object::{ObjectPlacementFace, ObjectRegistry},
    gameplay::availability::world_interaction_available,
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::{HOTBAR_INVENTORY_OFFSET, INVENTORY_SLOT_COUNT, PlayerHotbar},
        item_stack::ItemStack,
        viewmodel::ViewModelAnimation,
    },
    voxel::{
        edit::VoxelTopologyRuntime,
        object::ObjectCell,
        read::{VoxelRead, VoxelTopologyRead},
        texture_rotation::TextureRotation,
    },
    world_items::WorldItemSpawnRequest,
    world_objects::{WorldObjectPlaceRequest, detached_object_drop_request},
};

use super::block::{BlockTargetingSet, TargetedBlock};

const PEBBLE_ID: &str = "asteria:pebble";
const STONE_ID: &str = "asteria:stone";
const RUSTIC_WORKBENCH_ID: &str = "asteria:rustic_workbench";
const RUSTIC_WORKBENCH_PEBBLE_COST: u32 = 5;

pub(crate) struct RusticWorkbenchPlugin;

impl Plugin for RusticWorkbenchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            craft_rustic_workbench
                .after(BlockTargetingSet::PlacementState)
                .before(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

#[derive(SystemParam)]
struct RusticWorkbenchContext<'w> {
    targeted: ResMut<'w, TargetedBlock>,
    hotbar: ResMut<'w, PlayerHotbar>,
    objects: Res<'w, ObjectRegistry>,
    runtime: VoxelTopologyRuntime<'w>,
    object_placements: MessageWriter<'w, WorldObjectPlaceRequest>,
    item_spawns: MessageWriter<'w, WorldItemSpawnRequest>,
    viewmodel_animation: ResMut<'w, ViewModelAnimation>,
}

fn craft_rustic_workbench(
    buttons: Res<ButtonInput<MouseButton>>,
    player: Single<(&Transform, &GameMode), With<GameplayCamera>>,
    mut context: RusticWorkbenchContext,
) {
    let (_, game_mode) = player.into_inner();
    if !buttons.just_pressed(MouseButton::Right) || *game_mode != GameMode::Survival {
        return;
    }

    let Some(hit) = context.targeted.0 else {
        return;
    };
    if hit.block_id != STONE_ID {
        return;
    }

    let selected_slot = context.hotbar.selected_slot();
    if context.hotbar.item_at(selected_slot) != Some(PEBBLE_ID)
        || inventory_quantity(&context.hotbar, PEBBLE_ID) < RUSTIC_WORKBENCH_PEBBLE_COST
    {
        return;
    }

    let Some(definition) = context.objects.get(RUSTIC_WORKBENCH_ID) else {
        warn!("rustic workbench definition is missing");
        return;
    };
    if !definition.supports_placement_face(ObjectPlacementFace::Top) {
        warn!("rustic workbench must support top-face placement");
        return;
    }

    // World objects are attached to a solid support voxel. Replacing the stone
    // therefore anchors the table to the block directly below it, so the model
    // occupies exactly the voxel that used to contain the stone.
    let support = hit.voxel + IVec3::NEG_Y;
    let support_available = {
        let read = context.runtime.read();
        read.cell_at(support).is_some() && read.object_at(support).is_none()
    };
    if !support_available {
        return;
    }

    let replacement = ObjectCell::new(
        RUSTIC_WORKBENCH_ID,
        ObjectPlacementFace::Top,
        TextureRotation::default(),
    );

    let Some(mutation) = context.runtime.set_block_detailed(hit.voxel, None) else {
        return;
    };

    context.object_placements.write(WorldObjectPlaceRequest {
        support,
        object: replacement,
    });

    let consumed = consume_inventory_quantity(
        &mut context.hotbar,
        PEBBLE_ID,
        RUSTIC_WORKBENCH_PEBBLE_COST,
    );
    debug_assert!(
        consumed,
        "validated rustic workbench crafting must consume exactly five pebbles"
    );

    if let Some(detached) = mutation.detached_object
        && let Some(drop) = detached_object_drop_request(
            hit.voxel,
            mutation.previous_cell,
            detached,
            &context.objects,
        )
    {
        context.item_spawns.write(drop);
    }

    log_gameplay_event(format!(
        "workbench.craft workbench={} source_block={} ingredient={} quantity={} voxel={:?}",
        RUSTIC_WORKBENCH_ID,
        STONE_ID,
        PEBBLE_ID,
        RUSTIC_WORKBENCH_PEBBLE_COST,
        hit.voxel
    ));
    context.targeted.0 = None;
    context.viewmodel_animation.play_place();
}

fn inventory_quantity(hotbar: &PlayerHotbar, item_id: &str) -> u32 {
    (0..INVENTORY_SLOT_COUNT)
        .filter_map(|index| hotbar.inventory_stack_at(index))
        .filter(|stack| stack.id() == item_id)
        .map(ItemStack::quantity)
        .sum()
}

fn consume_inventory_quantity(
    hotbar: &mut PlayerHotbar,
    item_id: &str,
    quantity: u32,
) -> bool {
    if quantity == 0 {
        return true;
    }
    if inventory_quantity(hotbar, item_id) < quantity {
        return false;
    }

    let selected_index = HOTBAR_INVENTORY_OFFSET + hotbar.selected_slot();
    let mut remaining = quantity;
    let indices = std::iter::once(selected_index)
        .chain((0..INVENTORY_SLOT_COUNT).filter(|index| *index != selected_index));

    for index in indices {
        if remaining == 0 {
            break;
        }

        let Some(stack) = hotbar.inventory_stack_at(index).cloned() else {
            continue;
        };
        if stack.id() != item_id {
            continue;
        }

        let removed = remaining.min(stack.quantity());
        let replacement = if removed == stack.quantity() {
            None
        } else {
            let quantity = stack.quantity() - removed;
            Some(stack.with_quantity(quantity))
        };
        hotbar.replace_inventory_item(index, replacement);
        remaining -= removed;
    }

    debug_assert_eq!(remaining, 0);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::hotbar::BACKPACK_SLOT_COUNT;

    #[test]
    fn inventory_quantity_counts_backpack_and_hotbar() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.set_selected_stack(Some(ItemStack::new(PEBBLE_ID).with_quantity(2)));
        hotbar.replace_inventory_item(0, Some(ItemStack::new(PEBBLE_ID).with_quantity(4)));

        assert_eq!(inventory_quantity(&hotbar, PEBBLE_ID), 6);
    }

    #[test]
    fn workbench_cost_consumes_selected_stack_first_then_backpack() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.set_selected_stack(Some(ItemStack::new(PEBBLE_ID).with_quantity(2)));
        hotbar.replace_inventory_item(0, Some(ItemStack::new(PEBBLE_ID).with_quantity(4)));

        assert!(consume_inventory_quantity(
            &mut hotbar,
            PEBBLE_ID,
            RUSTIC_WORKBENCH_PEBBLE_COST
        ));
        assert!(hotbar.stack_at(hotbar.selected_slot()).is_none());
        assert_eq!(
            hotbar.inventory_stack_at(0).map(ItemStack::quantity),
            Some(1)
        );
    }

    #[test]
    fn insufficient_pebbles_are_not_consumed() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.set_selected_stack(Some(ItemStack::new(PEBBLE_ID).with_quantity(2)));
        hotbar.replace_inventory_item(
            BACKPACK_SLOT_COUNT - 1,
            Some(ItemStack::new(PEBBLE_ID).with_quantity(2)),
        );

        assert!(!consume_inventory_quantity(
            &mut hotbar,
            PEBBLE_ID,
            RUSTIC_WORKBENCH_PEBBLE_COST
        ));
        assert_eq!(inventory_quantity(&hotbar, PEBBLE_ID), 4);
    }
}
