use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry},
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    voxel::{coordinates::split_dimension_position, world::VoxelWorld},
};

use super::{
    biome_field::BiomeField,
    chunk_rendering::{spawn_chunk_mesh, TerrainMaterial},
    render_distance::{chunk_coords_in_cylinder, RenderDistanceSettings},
    test_world::{
        build_test_chunk, TERRAIN_MAX_CHUNK_Y, TERRAIN_MIN_CHUNK_Y,
    },
};

const CHUNKS_PER_FRAME: usize = 8;

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
    blocks: Res<BlockRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    material: Res<TerrainMaterial>,
    render_distance: Res<RenderDistanceSettings>,
    mut world: ResMut<VoxelWorld>,
    mut streaming: ResMut<ChunkStreamingState>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec2::new(player_chunk.x, player_chunk.z);
    let radius = render_distance.chunks();

    if streaming.center != Some(center) || streaming.render_distance != radius {
        rebuild_queue(&mut streaming, &world, center, radius);
    }

    for _ in 0..CHUNKS_PER_FRAME {
        let Some(coord) = streaming.pending.pop_front() else {
            break;
        };

        if world.has_generated_chunk(coord) {
            continue;
        }

        let chunk = build_test_chunk(coord, &blocks);
        world.insert_chunk(coord, chunk);

        let chunk = world
            .chunk(coord)
            .unwrap_or_else(|| panic!("streamed chunk should exist at {coord:?}"));
        spawn_chunk_mesh(
            &mut commands,
            &mut meshes,
            &world,
            coord,
            chunk,
            &biomes,
            &biome_field,
            &material.0,
        );
    }
}

fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    world: &VoxelWorld,
    center: IVec2,
    radius: i32,
) {
    let center_3d = IVec3::new(center.x, TERRAIN_MIN_CHUNK_Y, center.y);
    let coords = chunk_coords_in_cylinder(
        center_3d,
        radius,
        TERRAIN_MIN_CHUNK_Y,
        TERRAIN_MAX_CHUNK_Y,
    );
    let desired: HashSet<_> = coords.iter().copied().collect();
    let mut newly_exposed = Vec::new();
    let mut remaining = Vec::new();

    for coord in coords {
        if world.has_generated_chunk(coord) {
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
