use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{
        crash_log::{log_gameplay_event, log_gameplay_warn},
        keybinds::{KeybindAction, Keybinds},
    },
    content::{
        object::ObjectRegistry,
        world_recipe::{WorldRecipeIngredientDefinition, WorldRecipeRegistry},
    },
    gameplay::availability::world_interaction_available,
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::{HOTBAR_INVENTORY_OFFSET, INVENTORY_SLOT_COUNT, PlayerHotbar},
        item_stack::ItemStack,
        viewmodel::ViewModelAnimation,
    },
    targeting::block::{BlockTargetingSet, TargetedBlock},
    voxel::{
        edit::VoxelTopologyRuntime,
        object::ObjectCell,
        read::{VoxelRead, VoxelTopologyRead},
        texture_rotation::TextureRotation,
    },
    world_items::WorldItemSpawnRequest,
    world_objects::{WorldObjectPlaceRequest, detached_object_drop_request},
};

pub(crate) struct WorldCraftingPlugin;

impl Plugin for WorldCraftingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_world_recipe
                .after(BlockTargetingSet::PlacementState)
                .before(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

#[derive(SystemParam)]
struct WorldCraftingContext<'w> {
    targeted: ResMut<'w, TargetedBlock>,
    hotbar: ResMut<'w, PlayerHotbar>,
    recipes: Res<'w, WorldRecipeRegistry>,
    objects: Res<'w, ObjectRegistry>,
    runtime: VoxelTopologyRuntime<'w>,
    object_placements: MessageWriter<'w, WorldObjectPlaceRequest>,
    item_spawns: MessageWriter<'w, WorldItemSpawnRequest>,
    viewmodel_animation: ResMut<'w, ViewModelAnimation>,
}

fn apply_world_recipe(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    player: Single<&GameMode, With<GameplayCamera>>,
    mut context: WorldCraftingContext,
) {
    let game_mode = player.into_inner();
    let interaction_override = keys.pressed(keybinds.key_code(KeybindAction::Descend));
    if !buttons.just_pressed(MouseButton::Right)
        || game_mode.is_spectator()
        || interaction_override
    {
        return;
    }

    let Some(hit) = context.targeted.0 else {
        return;
    };
    let selected_slot = context.hotbar.selected_slot();
    let Some(held_item) = context.hotbar.item_at(selected_slot) else {
        return;
    };
    let Some(recipe) = context.recipes.matching(hit.block_id, held_item) else {
        return;
    };
    if !has_ingredients(&context.hotbar, &recipe.ingredients) {
        log_gameplay_event(format!(
            "world_recipe.reject recipe={} target_block={} held_item={} voxel={:?} reason=missing_ingredients",
            recipe.id, hit.block_id, held_item, hit.voxel
        ));
        return;
    }

    let recipe_id = recipe.id.clone();
    let ingredients = recipe.ingredients.clone();
    let (object_id, placement_face) = recipe.result.object_placement();
    let object_id = object_id.to_owned();

    let Some(definition) = context.objects.get(&object_id) else {
        log_gameplay_warn(format!(
            "world_recipe.reject recipe={} target_block={} result_object={} voxel={:?} reason=missing_result_object_definition",
            recipe_id, hit.block_id, object_id, hit.voxel
        ));
        return;
    };
    if !definition.supports_placement_face(placement_face) {
        log_gameplay_warn(format!(
            "world_recipe.reject recipe={} result_object={} voxel={:?} face={:?} reason=unsupported_placement_face",
            recipe_id, object_id, hit.voxel, placement_face
        ));
        return;
    }

    let support = hit.voxel - placement_face.normal();
    let support_available = {
        let read = context.runtime.read();
        read.cell_at(support).is_some() && read.object_at(support).is_none()
    };
    if !support_available {
        log_gameplay_event(format!(
            "world_recipe.reject recipe={} result_object={} voxel={:?} support={:?} reason=support_unavailable",
            recipe_id, object_id, hit.voxel, support
        ));
        return;
    }

    let replacement = ObjectCell::new(
        &object_id,
        placement_face,
        TextureRotation::default(),
    );
    let Some(mutation) = context.runtime.set_block_detailed(hit.voxel, None) else {
        log_gameplay_warn(format!(
            "world_recipe.reject recipe={} target_block={} voxel={:?} reason=block_mutation_rejected",
            recipe_id, hit.block_id, hit.voxel
        ));
        return;
    };

    context.object_placements.write(WorldObjectPlaceRequest {
        support,
        object: replacement,
    });
    consume_ingredients(&mut context.hotbar, &ingredients);

    for detached in mutation.detached_objects {
        if let Some(drop) = detached_object_drop_request(
            hit.voxel,
            mutation.previous_cell,
            detached,
            &context.objects,
        ) {
            context.item_spawns.write(drop);
        }
    }

    log_gameplay_event(format!(
        "world_recipe.apply recipe={} target_block={} result_object={} voxel={:?}",
        recipe_id, hit.block_id, object_id, hit.voxel
    ));
    context.targeted.0 = None;
    context.viewmodel_animation.play_place();
}

fn has_ingredients(hotbar: &PlayerHotbar, ingredients: &[WorldRecipeIngredientDefinition]) -> bool {
    ingredients
        .iter()
        .all(|ingredient| inventory_quantity(hotbar, &ingredient.item) >= ingredient.quantity)
}

fn consume_ingredients(hotbar: &mut PlayerHotbar, ingredients: &[WorldRecipeIngredientDefinition]) {
    for ingredient in ingredients {
        let consumed = consume_inventory_quantity(hotbar, &ingredient.item, ingredient.quantity);
        debug_assert!(
            consumed,
            "validated world recipe ingredients must be consumed atomically"
        );
    }
}

fn inventory_quantity(hotbar: &PlayerHotbar, item_id: &str) -> u32 {
    (0..INVENTORY_SLOT_COUNT)
        .filter_map(|index| hotbar.inventory_stack_at(index))
        .filter(|stack| stack.id() == item_id)
        .map(ItemStack::quantity)
        .sum()
}

fn consume_inventory_quantity(hotbar: &mut PlayerHotbar, item_id: &str, quantity: u32) -> bool {
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

        let stack_quantity = stack.quantity();
        let removed = remaining.min(stack_quantity);
        let replacement = if removed == stack_quantity {
            None
        } else {
            Some(stack.with_quantity(stack_quantity - removed))
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

    const PEBBLE_ID: &str = "asteria:pebble";
    const STICK_ID: &str = "asteria:stick";

    #[test]
    fn ingredient_check_counts_backpack_and_hotbar() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.set_selected_stack(Some(ItemStack::new(PEBBLE_ID).with_quantity(2)));
        hotbar.replace_inventory_item(0, Some(ItemStack::new(PEBBLE_ID).with_quantity(4)));

        assert!(has_ingredients(
            &hotbar,
            &[WorldRecipeIngredientDefinition {
                item: PEBBLE_ID.to_owned(),
                quantity: 6,
            }]
        ));
    }

    #[test]
    fn recipe_cost_consumes_selected_stack_first_then_backpack() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.set_selected_stack(Some(ItemStack::new(PEBBLE_ID).with_quantity(2)));
        hotbar.replace_inventory_item(0, Some(ItemStack::new(PEBBLE_ID).with_quantity(4)));

        consume_ingredients(
            &mut hotbar,
            &[WorldRecipeIngredientDefinition {
                item: PEBBLE_ID.to_owned(),
                quantity: 5,
            }],
        );

        assert!(hotbar.stack_at(hotbar.selected_slot()).is_none());
        assert_eq!(
            hotbar.inventory_stack_at(0).map(ItemStack::quantity),
            Some(1)
        );
    }

    #[test]
    fn multiple_ingredients_must_all_exist_before_recipe_can_run() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.set_selected_stack(Some(ItemStack::new(PEBBLE_ID).with_quantity(5)));
        hotbar.replace_inventory_item(
            BACKPACK_SLOT_COUNT - 1,
            Some(ItemStack::new(STICK_ID).with_quantity(1)),
        );

        let ingredients = [
            WorldRecipeIngredientDefinition {
                item: PEBBLE_ID.to_owned(),
                quantity: 5,
            },
            WorldRecipeIngredientDefinition {
                item: STICK_ID.to_owned(),
                quantity: 2,
            },
        ];
        assert!(!has_ingredients(&hotbar, &ingredients));
        assert_eq!(inventory_quantity(&hotbar, PEBBLE_ID), 5);
        assert_eq!(inventory_quantity(&hotbar, STICK_ID), 1);
    }
}
