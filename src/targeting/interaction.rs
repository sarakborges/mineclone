use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{block::BlockRegistry, tool::ToolRegistry},
    gameplay::availability::world_interaction_available,
    creatures::{CreatureAnimationState, CreatureInstance, EntityHealth},
    player::{camera::GameplayCamera, game_mode::GameMode, hotbar::PlayerHotbar, viewmodel::ViewModelAnimation},
    voxel::{
        cell::VoxelCell, edit::VoxelTopologyRuntime, raycast::VoxelHit,
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
    tools: Res<'w, ToolRegistry>,
}

fn edit_targeted_block(
    mut input: BlockEditInput,
    definitions: BlockEditDefinitions,
    mut runtime: VoxelTopologyRuntime,
    mut tool_uses: MessageWriter<ToolUse>,
    mut viewmodel_animation: ResMut<ViewModelAnimation>,
    mut creature_health: Query<(&mut EntityHealth, &mut CreatureAnimationState), With<CreatureInstance>>,
    mut commands: Commands,
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

    if left_pressed && let Some(entity) = input.creature_target.0 {
        if let Ok((mut health, mut animation)) = creature_health.get_mut(entity) {
            let dead = health.damage(1.0);
            animation.0 = if dead { "death" } else { "hurt" }.to_owned();
            viewmodel_animation.play_break();
            if dead {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    if let Some(tool_id) = selected_item.filter(|item_id| definitions.tools.get(item_id).is_some()) {
        if left_pressed {
            tool_uses.write(ToolUse {
                tool_id,
                button: ToolUseButton::Left,
                target: input.targeted.0,
            });
        }
        if right_pressed {
            tool_uses.write(ToolUse {
                tool_id,
                button: ToolUseButton::Right,
                target: input.targeted.0,
            });
        }
        return;
    }

    let Some(hit) = input.targeted.0 else {
        return;
    };

    let (edited, placed) = if left_pressed {
        (runtime.set_block(hit.voxel, None), false)
    } else {
        let Some(block_id) = selected_item else {
            return;
        };
        let Some(voxel) = placement_voxel(hit, runtime.world(), player_transform.translation) else {
            return;
        };
        let Some(block) = definitions.blocks.get(block_id) else {
            return;
        };
        let texture_rotation = TextureRotation::for_position(voxel, block.rotate_texture.any());
        let orientation = input.placement_orientation.for_block(selected_slot, block);
        let cell = VoxelCell::oriented(block_id, texture_rotation, orientation);

        (runtime.set_block(voxel, Some(cell)), true)
    };

    if edited.is_none() {
        return;
    }

    if placed {
        viewmodel_animation.play_place();
    } else {
        viewmodel_animation.play_break();
    }

    input.targeted.0 = None;
}
