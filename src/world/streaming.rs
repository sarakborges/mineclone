use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        dimension::DimensionRegistry,
        fluid::FluidRegistry,
    },
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    voxel::{coordinates::split_dimension_position, world::VoxelWorld},
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{
        refresh_adjacent_chunk_meshes, spawn_chunk_mesh, ChunkRenderPool, FluidMaterials,
        TerrainMaterials,
    },
    dimension::CurrentDimension,
    generation::generate_chunk,
    render_distance::{chunk_coords_in_volume, RenderDistanceSettings},
};

const CHUNKS_PER_FRAME: usize = 2;

#[derive(Resource, Default)]
pub struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_render_distance: i32,
    vertical_render_distance: i32,
    pending: VecDeque<IVec3>,
}

pub fn reset_chunk_streaming(mut state: ResMut<ChunkStreamingState>) {
    *state = ChunkStreamingState::default();
}

pub fn stream_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    player: Single<&Transform, With<GameplayCamera>>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    blocks: Res<BlockRegistry>,
    fluids: Res<FluidRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    terrain_materials: Res<TerrainMaterials>,
    fluid_materials: Res<FluidMaterials>,
    render_distance: Res<RenderDistanceSettings>,
    mut world: ResMut<VoxelWorld>,
    mut streaming: ResMut<ChunkStreamingState>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = render_distance.chunks();
    let vertical_radius = render_distance.vertical_chunks();

    if streaming.center != Some(center)
        || streaming.horizontal_render_distance != horizontal_radius
        || streaming.vertical_render_distance != vertical_radius
    {
        rebuild_queue(
            &mut streaming,
            &render_pool,
            center,
            horizontal_radius,
            vertical_radius,
        );
    }

    for _ in 0..CHUNKS_PER_FRAME {
        let Some(coord) = streaming.pending.pop_front() else {
            break;
        };

        if render_pool.contains(coord) {
            continue;
        }

        if world.has_generated_chunk(coord) {
            assert!(
                world.restore_chunk(coord),
                "generated chunk must be resident or archived: {coord:?}"
            );
        } else {
            let chunk = generate_chunk(
                coord,
                &blocks,
                &fluids,
                dimension,
                &biomes,
                &biome_field,
            );
            world.insert_chunk(coord, chunk);
        }

        let chunk = world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        spawn_chunk_mesh(
            &mut commands,
            &mut meshes,
            &mut render_pool,
            &world,
            coord,
            chunk,
            &biomes,
            &biome_field,
            &terrain_materials,
            &fluid_materials,
        );
        refresh_adjacent_chunk_meshes(
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
}

fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    render_pool: &ChunkRenderPool,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
) {
    let coords = chunk_coords_in_volume(center, horizontal_radius, vertical_radius);

    streaming.center = Some(center);
    streaming.horizontal_render_distance = horizontal_radius;
    streaming.vertical_render_distance = vertical_radius;
    streaming.pending = coords
        .into_iter()
        .filter(|coord| !render_pool.contains(*coord))
        .collect();
}
