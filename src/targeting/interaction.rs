use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        attack::AttackRegistry, block::BlockRegistry,
        builtin_ids::BIOME_TINT_METADATA_KEY,
        layer::{LayerFace, LayerRegistry},
        object::{ObjectInteraction, ObjectPlacementFace, ObjectRegistry}, player::PlayerDefinition,
        tool::ToolRegistry,
        tool_behavior::{MINE_TOOL_BEHAVIOR_ID, NONE_TOOL_BEHAVIOR_ID},
    },
    creatures::CreatureAttackRuntime,
    gameplay::availability::world_interaction_available,
    player::{
        camera::GameplayCamera, game_mode::GameMode, hotbar::PlayerHotbar,
        item_stack::ItemStack, viewmodel::ViewModelAnimation,
    },
    voxel::{
        cell::VoxelCell, edit::VoxelTopologyRuntime, layer::LayerCell, object::ObjectCell,
        log_variant::is_hollow_log_id,
        raycast::VoxelHit, texture_rotation::TextureRotation,
    },
    world_items::TargetedWorldItem,
    world_objects::{
        TargetedWorldObject, WorldObjectInstance, WorldObjectPlaceRequest,
        WorldObjectRemoveRequest,
    },
};

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
    placement_orientation::PlacementOrientation,
};

#[derive(Message, Clone)]
pub(crate) struct ToolUse {
    pub behavior_id: String,
    pub target: Option<VoxelHit>,
}

pub struct BlockInteractionPlugin;

impl Plugin for BlockInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ToolUse>().add_systems(
            Update,
            edit_targeted_block
                .in_set(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

#[derive(SystemParam)]
struct BlockEditInput<'w, 's> {
    buttons: Res<'w, ButtonInput<MouseButton>>,
    hotbar: ResMut<'w, PlayerHotbar>,
    placement_orientation: Res<'w, PlacementOrientation>,
    player: Single<'w, 's, (&'static Transform, &'static GameMode), With<GameplayCamera>>,
    targeted: ResMut<'w, TargetedBlock>,
    creature_target: Res<'w, super::block::TargetedCreature>,
    world_item_target: Res<'w, TargetedWorldItem>,
    object_target: ResMut<'w, TargetedWorldObject>,
    object_instances: Query<'w, 's, &'static WorldObjectInstance>,
}

#[derive(SystemParam)]
struct BlockEditDefinitions<'w> {
    blocks: Res<'w, BlockRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
    attacks: Res<'w, AttackRegistry>,
    player: Res<'w, PlayerDefinition>,
}

#[derive(SystemParam)]
struct BlockEditActions<'w> {
    object_placements: MessageWriter<'w, WorldObjectPlaceRequest>,
    object_removals: MessageWriter<'w, WorldObjectRemoveRequest>,
    tool_uses: MessageWriter<'w, ToolUse>,
    viewmodel_animation: ResMut<'w, ViewModelAnimation>,
}

#[derive(Clone, Copy)]
struct TargetedVoxelEdit<'a> {
    left_pressed: bool,
    right_pressed: bool,
    selected_slot: usize,
    selected_item: Option<&'static str>,
    biome_tint: Option<&'a str>,
    hit: VoxelHit,
    player_position: Vec3,
    game_mode: &'a GameMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VoxelEditOutcome {
    Consumed,
    LayerPlaced,
    BlockPlaced,
    BlockBroken,
}

fn edit_targeted_block(
    mut input: BlockEditInput,
    definitions: BlockEditDefinitions,
    mut runtime: VoxelTopologyRuntime,
    mut actions: BlockEditActions,
    mut creature_attack: CreatureAttackRuntime,
) {
    let left_pressed = input.buttons.just_pressed(MouseButton::Left);
    let right_pressed = input.buttons.just_pressed(MouseButton::Right);
    let middle_pressed = input.buttons.just_pressed(MouseButton::Middle);

    if !left_pressed && !right_pressed && !middle_pressed {
        return;
    }

    let (player_transform, game_mode) = input.player.into_inner();
    if input.world_item_target.0.is_some() {
        return;
    }
    if middle_pressed && game_mode.has_creative_inventory() {
        if let Some(entity) = input.object_target.0
            && let Ok(instance) = input.object_instances.get(entity)
            && definitions.objects.get(instance.object_id()).is_some()
        {
            input
                .hotbar
                .set_selected_stack(Some(ItemStack::new(instance.object_id())));
            return;
        }

        if let Some(hit) = input.targeted.0
            && definitions.blocks.get(hit.block_id).is_some()
        {
            let mut stack = ItemStack::new(hit.block_id);
            if let Some(cell) = runtime.world().cell_at(hit.voxel)
                && let Some(biome_id) = cell.secondary_property(BIOME_TINT_METADATA_KEY)
            {
                stack = stack.with_metadata(BIOME_TINT_METADATA_KEY, biome_id);
            }
            input.hotbar.set_selected_stack(Some(stack));
        }
        return;
    }

    let selected_slot = input.hotbar.selected_slot();
    let selected_item = input.hotbar.item_at(selected_slot);
    let selected_biome_tint = input
        .hotbar
        .stack_at(selected_slot)
        .and_then(|stack| stack.metadata().get(BIOME_TINT_METADATA_KEY))
        .map(str::to_owned);

    if let Some(entity) = input.object_target.0
        && let Ok(instance) = input.object_instances.get(entity)
        && let Some(definition) = definitions.objects.get(instance.object_id())
    {
        match definition.interaction {
            ObjectInteraction::Pickup if right_pressed => {
                if input
                    .hotbar
                    .try_insert_stack(ItemStack::new(instance.object_id()))
                    .is_ok()
                {
                    actions.object_removals.write(WorldObjectRemoveRequest {
                        entity,
                        drop_loot: false,
                    });
                    input.object_target.0 = None;
                    actions.viewmodel_animation.play_place();
                }
                return;
            }
            ObjectInteraction::Pickup if left_pressed => return,
            ObjectInteraction::Break if left_pressed => {
                actions.object_removals.write(WorldObjectRemoveRequest {
                    entity,
                    drop_loot: *game_mode == GameMode::Survival,
                });
                input.object_target.0 = None;
                actions.viewmodel_animation.play_break();
                return;
            }
            ObjectInteraction::Break if right_pressed => return,
            _ => {}
        }
    }

    // Left-clicking empty space still swings the player's arm. Mining tools use
    // the break swing; everything else uses the generic hit swing. World edits
    // and damage remain target-dependent below.
    if left_pressed && input.creature_target.0.is_none() && input.targeted.0.is_none() {
        let mining_tool = selected_item
            .and_then(|item_id| definitions.tools.get(item_id))
            .is_some_and(|tool| tool.left_behavior == MINE_TOOL_BEHAVIOR_ID);
        if mining_tool {
            actions.viewmodel_animation.play_break();
        } else {
            actions.viewmodel_animation.play_hit();
        }
    }

    if left_pressed && let Some(entity) = input.creature_target.0 {
        let Some(attack) = definitions.attacks.get(&definitions.player.attack) else {
            return;
        };
        if creature_attack.apply(entity, attack, player_transform.translation) {
            actions.viewmodel_animation.play_hit();
        }
        return;
    }

    if dispatch_selected_tool(
        selected_item,
        left_pressed,
        right_pressed,
        input.targeted.0,
        &definitions.tools,
        &mut actions.tool_uses,
    ) {
        return;
    }

    let Some(hit) = input.targeted.0 else {
        return;
    };

    if right_pressed
        && let Some(object_id) = selected_item
            .filter(|item_id| definitions.objects.get(item_id).is_some())
    {
        let definition = definitions
            .objects
            .get(object_id)
            .expect("selected object definition must exist");
        if let Some((support, face)) =
            object_placement_attachment(hit, definition, runtime.world())
        {
            actions.object_placements.write(WorldObjectPlaceRequest {
                support,
                object: ObjectCell::new(
                    object_id,
                    face,
                    TextureRotation::for_position(support, true),
                ),
            });
            consume_survival_placement(&mut input.hotbar, *game_mode);
            actions.viewmodel_animation.play_place();
        }
        return;
    }

    let outcome = edit_targeted_voxel(
        TargetedVoxelEdit {
            left_pressed,
            right_pressed,
            selected_slot,
            selected_item,
            biome_tint: selected_biome_tint.as_deref(),
            hit,
            player_position: player_transform.translation,
            game_mode,
        },
        &definitions.blocks,
        &definitions.layers,
        &input.placement_orientation,
        &mut runtime,
    );

    match outcome {
        VoxelEditOutcome::Consumed => {}
        VoxelEditOutcome::LayerPlaced => {
            consume_survival_placement(&mut input.hotbar, *game_mode);
            actions.viewmodel_animation.play_place();
        }
        VoxelEditOutcome::BlockPlaced => {
            consume_survival_placement(&mut input.hotbar, *game_mode);
            actions.viewmodel_animation.play_place();
            input.targeted.0 = None;
        }
        VoxelEditOutcome::BlockBroken => {
            actions.viewmodel_animation.play_break();
            input.targeted.0 = None;
        }
    }
}

fn consume_survival_placement(hotbar: &mut PlayerHotbar, game_mode: GameMode) {
    if game_mode != GameMode::Survival {
        return;
    }

    let consumed = hotbar.consume_selected_item();
    debug_assert!(
        consumed,
        "successful survival placement must consume the selected hotbar item"
    );
}

fn dispatch_selected_tool(
    selected_item: Option<&'static str>,
    left_pressed: bool,
    right_pressed: bool,
    target: Option<VoxelHit>,
    tools: &ToolRegistry,
    tool_uses: &mut MessageWriter<'_, ToolUse>,
) -> bool {
    let Some(tool) = selected_item.and_then(|tool_id| tools.get(tool_id)) else {
        return false;
    };

    let mut consumed = false;

    if left_pressed && tool.left_behavior != MINE_TOOL_BEHAVIOR_ID {
        consumed = true;
        if tool.left_behavior != NONE_TOOL_BEHAVIOR_ID {
            tool_uses.write(ToolUse {
                behavior_id: tool.left_behavior.clone(),
                target,
            });
        }
    }

    if right_pressed {
        consumed = true;
        if tool.right_behavior != NONE_TOOL_BEHAVIOR_ID {
            tool_uses.write(ToolUse {
                behavior_id: tool.right_behavior.clone(),
                target,
            });
        }
    }

    consumed
}

fn edit_targeted_voxel(
    request: TargetedVoxelEdit<'_>,
    blocks: &BlockRegistry,
    layers: &LayerRegistry,
    placement_orientation: &PlacementOrientation,
    runtime: &mut VoxelTopologyRuntime<'_>,
) -> VoxelEditOutcome {
    if request.right_pressed
        && let Some(layer_id) = request
            .selected_item
            .filter(|item_id| layers.get(item_id).is_some())
    {
        let Some(face) = LayerFace::from_normal(request.hit.normal) else {
            return VoxelEditOutcome::Consumed;
        };
        let layer = LayerCell::new(layer_id, TextureRotation::default());
        return if runtime
            .add_layer(request.hit.voxel, face, layer)
            .is_some()
        {
            VoxelEditOutcome::LayerPlaced
        } else {
            VoxelEditOutcome::Consumed
        };
    }

    if request.left_pressed {
        if matches!(request.game_mode, GameMode::Survival) {
            return VoxelEditOutcome::Consumed;
        }
        return if runtime.set_block(request.hit.voxel, None).is_some() {
            VoxelEditOutcome::BlockBroken
        } else {
            VoxelEditOutcome::Consumed
        };
    }

    let Some(block_id) = request.selected_item else {
        return VoxelEditOutcome::Consumed;
    };
    let Some(voxel) = placement_voxel(request.hit, runtime.world(), request.player_position) else {
        return VoxelEditOutcome::Consumed;
    };
    let Some(block) = blocks.get(block_id) else {
        return VoxelEditOutcome::Consumed;
    };
    let texture_rotation = TextureRotation::for_position(voxel, block.rotate_texture.any());
    let orientation = placement_orientation.for_block(request.selected_slot, block);
    let mut cell = VoxelCell::oriented(block_id, texture_rotation, orientation);
    if let Some(biome_id) = request.biome_tint {
        cell = cell.with_secondary_property(BIOME_TINT_METADATA_KEY, biome_id);
    }

    if runtime.set_block(voxel, Some(cell)).is_some() {
        VoxelEditOutcome::BlockPlaced
    } else {
        VoxelEditOutcome::Consumed
    }
}



fn object_placement_attachment(
    hit: VoxelHit,
    definition: &crate::content::object::ObjectDefinition,
    world: &crate::voxel::world::VoxelWorld,
) -> Option<(IVec3, ObjectPlacementFace)> {
    if definition.supports_placement_face(ObjectPlacementFace::Top) {
        if hollow_log_accepts_object(hit.voxel, definition, world) {
            return Some((hit.voxel, ObjectPlacementFace::Top));
        }

        let adjacent = hit.voxel + hit.normal;
        if adjacent.y >= 0 && hollow_log_accepts_object(adjacent, definition, world) {
            return Some((adjacent, ObjectPlacementFace::Top));
        }
    }

    let face = ObjectPlacementFace::from_normal(hit.normal)?;
    if !definition.supports_placement_face(face) {
        return None;
    }

    let object_space = hit.voxel + hit.normal;
    (object_space.y >= 0
        && world.is_loaded_at(hit.voxel)
        && world.is_loaded_at(object_space)
        && world.cell_at(object_space).is_none()
        && world.object_at(hit.voxel).is_none())
    .then_some((hit.voxel, face))
}

fn hollow_log_accepts_object(
    voxel: IVec3,
    definition: &crate::content::object::ObjectDefinition,
    world: &crate::voxel::world::VoxelWorld,
) -> bool {
    let Some(cell) = world.cell_at(voxel) else {
        return false;
    };
    if !is_hollow_log_id(cell.block_id)
        || world.object_at(voxel).is_some()
        || !world.is_loaded_at(voxel)
    {
        return false;
    }

    let cavity_diameter = 1.0 - crate::voxel::microblock::HOLLOW_LOG_WALL_THICKNESS * 2.0;
    let width = definition.target.size[0];
    let depth = definition.target.size[2];
    match cell.orientation {
        crate::content::block_orientation::BlockOrientation::X => depth <= cavity_diameter,
        crate::content::block_orientation::BlockOrientation::Y => {
            width <= cavity_diameter && depth <= cavity_diameter
        }
        crate::content::block_orientation::BlockOrientation::Z => width <= cavity_diameter,
    }
}
