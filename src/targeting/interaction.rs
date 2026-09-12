use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::{block::BlockRegistry, tool::ToolRegistry},
    player::{
        camera::GameplayCamera, hotbar::PlayerHotbar, inventory::InventoryState,
        viewmodel::ViewModelAnimation,
    },
    tools::BrushPaletteState,
    voxel::{
        cell::VoxelCell, lighting::PendingLightingUpdates, raycast::VoxelHit,
        texture_rotation::TextureRotation, world::VoxelWorld,
    },
    world::{
        chunk_remesh::ChunkRemeshQueue, chunk_system_params::ChunkContent,
        fluid_updates::PendingFluidUpdates,
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
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(PauseState::Running))
                .run_if(in_state(InventoryState::Closed))
                .run_if(in_state(BrushPaletteState::Closed)),
        );
    }
}

#[derive(SystemParam)]
struct BlockEditInput<'w, 's> {
    buttons: Res<'w, ButtonInput<MouseButton>>,
    hotbar: Res<'w, PlayerHotbar>,
    placement_orientation: Res<'w, PlacementOrientation>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    targeted: ResMut<'w, TargetedBlock>,
}

fn edit_targeted_block(
    mut input: BlockEditInput,
    content: ChunkContent,
    tools: Res<ToolRegistry>,
    mut tool_uses: MessageWriter<ToolUse>,
    mut viewmodel_animation: ResMut<ViewModelAnimation>,
    mut world: ResMut<VoxelWorld>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut fluid_updates: ResMut<PendingFluidUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let left_pressed = input.buttons.just_pressed(MouseButton::Left);
    let right_pressed = input.buttons.just_pressed(MouseButton::Right);

    if !left_pressed && !right_pressed {
        return;
    }

    let selected_slot = input.hotbar.selected_slot();
    let selected_item = input.hotbar.item_at(selected_slot);

    if let Some(tool_id) = selected_item.filter(|item_id| tools.get(item_id).is_some()) {
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

    let (edited_chunk, edited_voxel, placed) = if left_pressed {
        (world.set_block_at(hit.voxel, None), hit.voxel, false)
    } else {
        let Some(block_id) = selected_item else {
            return;
        };
        let Some(voxel) = placement_voxel(hit, &world, input.player.translation) else {
            return;
        };
        let Some(block) = content.blocks.get(block_id) else {
            return;
        };
        let texture_rotation = TextureRotation::for_position(voxel, block.rotate_texture.any());
        let orientation = input.placement_orientation.for_block(selected_slot, block);
        let cell = VoxelCell::oriented(block_id, texture_rotation, orientation);

        (world.set_block_at(voxel, Some(cell)), voxel, true)
    };

    let Some(coord) = edited_chunk else {
        return;
    };

    lighting.enqueue_voxel_edit(edited_voxel);
    fluid_updates.enqueue_voxel_edit(edited_voxel);

    // Geometry, face exposure, shadow casters and baked voxel lighting all
    // converge through the same post-lighting remesh path. The old transparent
    // fast-path rebuilt glass before lighting had updated, which could leave the
    // edited chunk stale until a later neighboring edit.
    remesh_queue.enqueue_voxel_edit(coord);

    if placed {
        viewmodel_animation.play_place();
    } else {
        viewmodel_animation.play_break();
    }

    input.targeted.0 = None;
}
