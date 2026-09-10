use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry, fluid::FluidRegistry},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::split_dimension_position, lighting::relight_after_chunk_unloads,
        world::VoxelWorld,
    },
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{
        ChunkRenderContext, ChunkRenderPool, FluidMaterials, TerrainMaterials, refresh_chunk_mesh,
    },
    render_distance::RenderDistanceSettings,
};

const CHUNK_NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy ECS system parameters declare independent unloading and rendering resources"
)]
pub fn unload_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    blocks: Res<BlockRegistry>,
    fluids: Res<FluidRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    terrain_materials: Res<TerrainMaterials>,
    fluid_materials: Res<FluidMaterials>,
    mut world: ResMut<VoxelWorld>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = render_distance.chunks();
    let vertical_radius = render_distance.vertical_chunks();
    let horizontal_radius_squared = horizontal_radius * horizontal_radius;
    let to_unload = render_pool
        .active_coords()
        .filter(|coord| {
            let delta = *coord - center;
            let outside_horizontal =
                delta.x * delta.x + delta.z * delta.z > horizontal_radius_squared;
            let outside_vertical = delta.y.abs() > vertical_radius;

            outside_horizontal || outside_vertical
        })
        .collect::<Vec<_>>();

    for coord in &to_unload {
        let Some((entities, mesh_handles)) = render_pool.take(*coord) else {
            continue;
        };

        for mesh_handle in mesh_handles {
            let _ = meshes.remove(&mesh_handle);
        }

        for entity in entities {
            commands.entity(entity).despawn();
        }

        world.archive_chunk(*coord);
    }

    let mut chunks_to_remesh =
        relight_after_chunk_unloads(&mut world, &to_unload, &blocks, &fluids);

    for coord in &to_unload {
        for offset in CHUNK_NEIGHBORS {
            let neighbor = *coord + offset;
            if render_pool.contains(neighbor) {
                chunks_to_remesh.insert(neighbor);
            }
        }
    }

    let render_context = ChunkRenderContext {
        world: &world,
        blocks: &blocks,
        biomes: &biomes,
        biome_field: &biome_field,
        terrain_materials: &terrain_materials,
        fluid_materials: &fluid_materials,
    };

    for coord in chunks_to_remesh {
        refresh_chunk_mesh(
            &mut commands,
            &mut meshes,
            &mut render_pool,
            coord,
            &render_context,
        );
    }
}
