use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    player::{camera::GameplayCamera, hotbar::PlayerHotbar, viewmodel::ViewModelAnimation},
    voxel::{
        cell::VoxelCell, lighting::PendingLightingUpdates, neighbors::CARDINAL_NEIGHBORS,
        texture_rotation::TextureRotation, world::VoxelWorld,
    },
    world::{chunk_remesh::ChunkRemeshQueue, chunk_system_params::ChunkContent},
};

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
};

pub struct BlockInteractionPlugin;

impl Plugin for BlockInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            edit_targeted_block
                .in_set(BlockTargetingSet::Interaction)
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(PauseState::Running)),
        );
    }
}

#[derive(SystemParam)]
struct BlockEditInput<'w, 's> {
    buttons: Res<'w, ButtonInput<MouseButton>>,
    hotbar: Res<'w, PlayerHotbar>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    targeted: ResMut<'w, TargetedBlock>,
}

fn edit_targeted_block(
    mut input: BlockEditInput,
    content: ChunkContent,
    mut viewmodel_animation: ResMut<ViewModelAnimation>,
    mut world: ResMut<VoxelWorld>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    let break_pressed = input.buttons.just_pressed(MouseButton::Left);
    let place_pressed = input.buttons.just_pressed(MouseButton::Right);

    if !break_pressed && !place_pressed {
        return;
    }

    let Some(hit) = input.targeted.0 else {
        return;
    };

    let (edited_chunk, edited_voxel, placed) = if break_pressed {
        (world.set_block_at(hit.voxel, None), hit.voxel, false)
    } else {
        let Some(block_id) = input.hotbar.item_at(input.hotbar.selected_slot()) else {
            return;
        };
        let Some(voxel) = placement_voxel(hit, &world, input.player.translation) else {
            return;
        };
        let Some(block) = content.blocks.get(block_id) else {
            return;
        };
        let rotation = TextureRotation::for_position(voxel, block.rotate_texture.any());

        (
            world.set_block_at(voxel, Some(VoxelCell::new(block_id, rotation))),
            voxel,
            true,
        )
    };

    let Some(coord) = edited_chunk else {
        return;
    };

    lighting.enqueue_voxel_edit(edited_voxel);

    remesh_queue.enqueue_priority(coord);
    for offset in CARDINAL_NEIGHBORS {
        remesh_queue.enqueue(coord + offset);
    }

    if placed {
        viewmodel_animation.play_place();
    } else {
        viewmodel_animation.play_break();
    }

    input.targeted.0 = None;
}
