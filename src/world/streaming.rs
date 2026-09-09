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
    voxel::{
        coordinates::split_dimension_position,
        lighting::initialize_chunk_lighting,
        world::VoxelWorld,
    },
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{
        refresh_adjacent_chunk_meshes, refresh_chunk_mesh, spawn_chunk_mesh, ChunkRenderPool,
        FluidMaterials, TerrainMaterials,
    },
    dimension::CurrentDimension,
    render_distance::{chunk_coords_in_cylinder, RenderDistanceSettings},
    terrain::{build_chunk, chunk_y_bounds},
};

const CHUNKS_PER_FRAME: usize = 2;

#[derive(Resource, Default)]
pub struct ChunkStreamingState {
    center: Option<IVec2>,
    render_distance: i32,
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
    let center = IVec2::new(player_chunk.x, player_chunk.z);
    let radius = render_distance.chunks();
    let (min_chunk_y, max_chunk_y) = chunk_y_bounds(dimension, &biomes);

    if streaming.center != Some(center) || streaming.render_distance != radius {
        rebuild_queue(
            &mut streaming,
            &render_pool,
            center,
            radius,
            min_chunk_y,
            max_chunk_y,
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
            let chunk = build_chunk(
                coord,
                &blocks,
                &fluids,
                dimension,
                &biomes,
                &biome_field,
            );
            world.insert_chunk(coord, chunk);
        }

        let lighting_changes = initialize_chunk_lighting(&mut world, coord, &blocks, &fluids);
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
            &blocks,
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
            &blocks,
            &biomes,
            &biome_field,
            &terrain_materials,
            &fluid_materials,
        );

        for changed_coord in lighting_changes {
            if changed_coord == coord {
                continue;
            }

            refresh_chunk_mesh(
                &mut commands,
                &mut meshes,
                &mut render_pool,
                &world,
                changed_coord,
                &blocks,
                &biomes,
                &biome_field,
                &terrain_materials,
                &fluid_materials,
            );
        }
    }
}

fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    render_pool: &ChunkRenderPool,
    center: IVec2,
    radius: i32,
    min_chunk_y: i32,
    max_chunk_y: i32,
) {
    let center_3d = IVec3::new(center.x, min_chunk_y, center.y);
    let coords = chunk_coords_in_cylinder(center_3d, radius, min_chunk_y, max_chunk_y);

    streaming.center = Some(center);
    streaming.render_distance = radius;
    streaming.pending = coords
        .into_iter()
        .filter(|coord| !render_pool.contains(*coord))
        .collect();
}
