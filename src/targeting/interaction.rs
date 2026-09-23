use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        attack::AttackRegistry, block::BlockRegistry, layer::{LayerFace, LayerRegistry},
        player::PlayerDefinition, tool::ToolRegistry,
    },
    creatures::CreatureAttackRuntime,
    gameplay::availability::world_interaction_available,
    player::{camera::GameplayCamera, game_mode::GameMode, hotbar::PlayerHotbar, viewmodel::ViewModelAnimation},
    voxel::{
        cell::VoxelCell, edit::VoxelTopologyRuntime, layer::LayerCell, raycast::VoxelHit,
        texture_rotation::TextureRotation,
    },
};

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
    placement_orientation::PlacementOrientation,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolUseButton {
    Left,
    Right,
}

#[derive(Message, Clone, Copy)]
pub(crate) struct ToolUse {
    pub tool_id: &'static str,
    pub button: ToolUseButton,
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
}

#[derive(SystemParam)]
struct BlockEditDefinitions<'w> {
    blocks: Res<'w, BlockRegistry>,
    layers: Res<'w, LayerRegistry>,
    tools: Res<'w, ToolRegistry>,
    attacks: Res<'w, AttackRegistry>,
    player: Res<'w, PlayerDefinition>,
}

#[derive(Clone, Copy)]
struct TargetedVoxelEdit<'a> {
    left_pressed: bool,
    right_pressed: bool,
    selected_slot: usize,
    selected_item: Option<&'static str>,
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
    mut tool_uses: MessageWriter<ToolUse>,
    mut viewmodel_animation: ResMut<ViewModelAnimation>,
    mut creature_attack: CreatureAttackRuntime,
) {
    let left_pressed = input.buttons.just_pressed(MouseButton::Left);
    let right_pressed = input.buttons.just_pressed(MouseButton::Right);
    let middle_pressed = input.buttons.just_pressed(MouseButton::Middle);

    if !left_pressed && !right_pressed && !middle_pressed {
        return;
    }

    let (player_transform, game_mode) = input.player.into_inner();
    if middle_pressed && game_mode.has_creative_inventory() {
        if let Some(hit) = input.targeted.0
            && definitions.blocks.get(hit.block_id).is_some()
        {
            input.hotbar.set_selected_item(Some(hit.block_id));
        }
        return;
    }

    let selected_slot = input.hotbar.selected_slot();
    let selected_item = input.hotbar.item_at(selected_slot);

    // Left-clicking empty space still swings the player's arm. Mining tools use
    // the break swing; everything else uses the generic hit swing. World edits
    // and damage remain target-dependent below.
    if left_pressed && input.creature_target.0.is_none() && input.targeted.0.is_none() {
        let mining_tool = selected_item
            .and_then(|item_id| definitions.tools.get(item_id))
            .is_some_and(|tool| tool.mining.is_mining_tool());
        if mining_tool {
            viewmodel_animation.play_break();
        } else {
            viewmodel_animation.play_hit();
        }
    }

    if left_pressed && let Some(entity) = input.creature_target.0 {
        let Some(attack) = definitions.attacks.get(&definitions.player.attack) else {
            return;
        };
        if creature_attack.apply(entity, attack, player_transform.translation) {
            viewmodel_animation.play_hit();
        }
        return;
    }

    if dispatch_selected_tool(
        selected_item,
        left_pressed,
        right_pressed,
        input.targeted.0,
        &definitions.tools,
        &mut tool_uses,
    ) {
        return;
    }

    let Some(hit) = input.targeted.0 else {
        return;
    };
    let outcome = edit_targeted_voxel(
        TargetedVoxelEdit {
            left_pressed,
            right_pressed,
            selected_slot,
            selected_item,
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
            viewmodel_animation.play_place();
        }
        VoxelEditOutcome::BlockPlaced => {
            viewmodel_animation.play_place();
            input.targeted.0 = None;
        }
        VoxelEditOutcome::BlockBroken => {
            viewmodel_animation.play_break();
            input.targeted.0 = None;
        }
    }
}

fn dispatch_selected_tool(
    selected_item: Option<&'static str>,
    left_pressed: bool,
    right_pressed: bool,
    target: Option<VoxelHit>,
    tools: &ToolRegistry,
    tool_uses: &mut MessageWriter<'_, ToolUse>,
) -> bool {
    let Some(tool_id) = selected_item else {
        return false;
    };
    let Some(tool) = tools.get(tool_id) else {
        return false;
    };
    let is_mining_tool = tool.mining.is_mining_tool();

    if left_pressed && !is_mining_tool {
        tool_uses.write(ToolUse {
            tool_id,
            button: ToolUseButton::Left,
            target,
        });
    }
    if right_pressed {
        tool_uses.write(ToolUse {
            tool_id,
            button: ToolUseButton::Right,
            target,
        });
    }

    right_pressed || (left_pressed && !is_mining_tool)
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
    let cell = VoxelCell::oriented(block_id, texture_rotation, orientation);

    if runtime.set_block(voxel, Some(cell)).is_some() {
        VoxelEditOutcome::BlockPlaced
    } else {
        VoxelEditOutcome::Consumed
    }
}

