use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::{biome::BiomeRegistry, block::BlockRegistry},
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    voxel::{cell::VoxelCell, texture_rotation::TextureRotation, world::VoxelWorld},
    world::{
        biome_field::BiomeField,
        chunk_rendering::{
            refresh_adjacent_chunk_meshes, spawn_chunk_mesh, ChunkRenderPool, FluidMaterials,
            TerrainMaterials,
        },
    },
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

fn edit_targeted_block(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<BlockRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    terrain_materials: Res<TerrainMaterials>,
    fluid_materials: Res<FluidMaterials>,
    hotbar: Res<PlayerHotbar>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut world: ResMut<VoxelWorld>,
    mut render_pool: ResMut<ChunkRenderPool>,
    mut targeted: ResMut<TargetedBlock>,
) {
    let Some(hit) = targeted.0 else {
        return;
    };

    let edited_chunk = if buttons.just_pressed(MouseButton::Left) {
        world.set_block_at(hit.voxel, None)
    } else if buttons.just_pressed(MouseButton::Right) {
        let Some(block_id) = hotbar.item_at(hotbar.selected_slot()) else {
            return;
        };
        let Some(voxel) = placement_voxel(hit, &world, player.translation) else {
            return;
        };
        let block = blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));
        let rotation = if block.rotate_texture {
            TextureRotation::default()
        } else {
            TextureRotation::default()
        };

        world.set_block_at(voxel, Some(VoxelCell::new(block_id, rotation)))
    } else {
        return;
    };

    let Some(coord) = edited_chunk else {
        return;
    };

    targeted.0 = None;
    remesh_edited_chunk(
        &mut commands,
        &mut meshes,
        &mut render_pool,
        &world,
        coord,
        &biomes,
        &biome_field,
        &terrain_materials,
        &fluid_materials,
    );
}

#[allow(clippy::too_many_arguments)]
fn remesh_edited_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    world: &VoxelWorld,
    coord: IVec3,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    terrain_materials: &TerrainMaterials,
    fluid_materials: &FluidMaterials,
) {
    if render_pool.contains(coord) {
        if let Some((entities, mesh_handles)) = render_pool.take(coord) {
            for entity in entities {
                commands.entity(entity).despawn();
            }

            for handle in mesh_handles {
                let _ = meshes.remove(&handle);
                render_pool.recycle_mesh_handle(handle);
            }
        }

        if let Some(chunk) = world.chunk(coord) {
            spawn_chunk_mesh(
                commands,
                meshes,
                render_pool,
                world,
                coord,
                chunk,
                biomes,
                biome_field,
                terrain_materials,
                fluid_materials,
            );
        }
    }

    refresh_adjacent_chunk_meshes(
        commands,
        meshes,
        render_pool,
        world,
        coord,
        biomes,
        biome_field,
        terrain_materials,
        fluid_materials,
    );
}
