use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::{biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry},
    player::{camera::GameplayCamera, hotbar::PlayerHotbar, viewmodel::ViewModelAnimation},
    voxel::{
        cell::VoxelCell, lighting::relight_after_voxel_edit, texture_rotation::TextureRotation,
        world::VoxelWorld,
    },
    world::{
        biome_field::BiomeField,
        chunk_rendering::{ChunkRenderPool, FluidMaterials, TerrainMaterials, refresh_chunk_mesh},
    },
};

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
};

const CHUNK_NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

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

fn edit_targeted_block(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<BlockRegistry>,
    fluids: Res<FluidRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    terrain_materials: Res<TerrainMaterials>,
    fluid_materials: Res<FluidMaterials>,
    hotbar: Res<PlayerHotbar>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut viewmodel_animation: ResMut<ViewModelAnimation>,
    mut world: ResMut<VoxelWorld>,
    mut render_pool: ResMut<ChunkRenderPool>,
    mut targeted: ResMut<TargetedBlock>,
) {
    let break_pressed = buttons.just_pressed(MouseButton::Left);
    let place_pressed = buttons.just_pressed(MouseButton::Right);

    if !break_pressed && !place_pressed {
        return;
    }

    let Some(hit) = targeted.0 else {
        return;
    };

    let (edited_chunk, edited_voxel, placed) = if break_pressed {
        (world.set_block_at(hit.voxel, None), hit.voxel, false)
    } else {
        let Some(block_id) = hotbar.item_at(hotbar.selected_slot()) else {
            return;
        };
        let Some(voxel) = placement_voxel(hit, &world, player.translation) else {
            return;
        };
        if blocks.get(block_id).is_none() {
            return;
        }

        (
            world.set_block_at(
                voxel,
                Some(VoxelCell::new(block_id, TextureRotation::default())),
            ),
            voxel,
            true,
        )
    };

    let Some(coord) = edited_chunk else {
        return;
    };

    let mut chunks_to_remesh = relight_after_voxel_edit(&mut world, edited_voxel, &blocks, &fluids);
    chunks_to_remesh.insert(coord);
    for offset in CHUNK_NEIGHBORS {
        chunks_to_remesh.insert(coord + offset);
    }

    if placed {
        viewmodel_animation.play_place();
    } else {
        viewmodel_animation.play_break();
    }

    targeted.0 = None;

    for chunk_coord in chunks_to_remesh {
        refresh_chunk_mesh(
            &mut commands,
            &mut meshes,
            &mut render_pool,
            &world,
            chunk_coord,
            &blocks,
            &biomes,
            &biome_field,
            &terrain_materials,
            &fluid_materials,
        );
    }
}
