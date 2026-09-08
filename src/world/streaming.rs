use std::collections::{HashSet, VecDeque};

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
    render_distance::{chunk_coords_in_cylinder, RenderDistanceSettings},
    terrain::{build_chunk, chunk_y_bounds},
};

const CHUNKS_PER_FRAME: usize = 4;

#[derive(Resource, Default)]
pub struct ChunkStreamingState {
    center: Option<IVec2>,
    render_distance: i32,
    desired: HashSet<IVec3>,
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
    center: IVec2,
    radius: i32,
    min_chunk_y: i32,
    max_chunk_y: i32,
) {
    let center_3d = IVec3::new(center.x, min_chunk_y, center.y);
    let coords = chunk_coords_in_cylinder(
        center_3d,
        radius,
        min_chunk_y,
        max_chunk_y,
    );
    let desired: HashSet<_> = coords.iter().copied().collect();
    let mut newly_exposed = Vec::new();
    let mut remaining = Vec::new();

    for coord in coords {
        if render_pool.contains(coord) {
            continue;
        }

        if streaming.desired.contains(&coord) {
            remaining.push(coord);
        } else {
            newly_exposed.push(coord);
        }
    }

    sort_by_distance(&mut newly_exposed, center);
    sort_by_distance(&mut remaining, center);

    streaming.center = Some(center);
    streaming.render_distance = radius;
    streaming.desired = desired;
    streaming.pending = newly_exposed.into_iter().chain(remaining).collect();
}

fn sort_by_distance(coords: &mut [IVec3], center: IVec2) {
    coords.sort_by_key(|coord| {
        let dx = coord.x - center.x;
        let dz = coord.z - center.y;
        dx * dx + dz * dz
    });
}
