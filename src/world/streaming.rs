use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, dimension::DimensionRegistry,
        fluid::FluidRegistry,
    },
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::split_dimension_position, lighting::initialize_chunk_lighting,
        world::VoxelWorld,
    },
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{
        ChunkRenderContext, ChunkRenderPool, FluidMaterials, TerrainMaterials,
        refresh_adjacent_chunk_meshes, refresh_chunk_mesh, spawn_chunk_mesh,
    },
    dimension::CurrentDimension,
    generation::generate_chunk,
    render_distance::{RenderDistanceSettings, chunk_coords_in_volume},
    world_feature_fields::WorldFeatureFields,
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

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy ECS system parameters declare independent streaming and rendering resources"
)]
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
    feature_fields: Res<WorldFeatureFields>,
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
                &feature_fields,
            );
            world.insert_chunk(coord, chunk);
        }

        let lighting_changes = initialize_chunk_lighting(&mut world, coord, &blocks, &fluids);
        let chunk = world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        let render_context = ChunkRenderContext {
            world: &world,
            blocks: &blocks,
            biomes: &biomes,
            biome_field: &biome_field,
            terrain_materials: &terrain_materials,
            fluid_materials: &fluid_materials,
        };

        spawn_chunk_mesh(
            &mut commands,
            &mut meshes,
            &mut render_pool,
            coord,
            chunk,
            &render_context,
        );
        refresh_adjacent_chunk_meshes(
            &mut commands,
            &mut meshes,
            &mut render_pool,
            coord,
            &render_context,
        );

        for changed_coord in lighting_changes {
            if changed_coord == coord {
                continue;
            }

            refresh_chunk_mesh(
                &mut commands,
                &mut meshes,
                &mut render_pool,
                changed_coord,
                &render_context,
            );
        }
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
