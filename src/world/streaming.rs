use std::collections::{HashMap, HashSet, VecDeque};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        chunk::CHUNK_SIZE, coordinates::split_dimension_position,
        lighting::initialize_chunk_lighting, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
};

use super::{
    biome_field::BiomeField,
    chunk_loading::ensure_chunk_loaded,
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::{ChunkRenderPool, spawn_chunk_mesh},
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::PendingFluidUpdates,
    render_distance::{RenderDistanceSettings, chunk_coords_in_volume},
    terrain::surface_height,
};

const CHUNKS_PER_FRAME: usize = 2;
const SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const SURFACE_PADDING_ABOVE_CHUNKS: i32 = 1;

#[derive(Resource, Default)]
pub struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_render_distance: i32,
    vertical_render_distance: i32,
    desired: HashSet<IVec3>,
    pending: VecDeque<IVec3>,
}

impl ChunkStreamingState {
    pub(crate) fn wants(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord)
    }
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingInputs<'w, 's> {
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    render_distance: Res<'w, RenderDistanceSettings>,
    world: ResMut<'w, VoxelWorld>,
    streaming: ResMut<'w, ChunkStreamingState>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub fn reset_chunk_streaming(mut state: ResMut<ChunkStreamingState>) {
    *state = ChunkStreamingState::default();
}

pub fn stream_chunks(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut inputs: ChunkStreamingInputs,
    mut fluid_updates: ResMut<PendingFluidUpdates>,
) {
    let feet_position = inputs.player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = inputs.render_distance.chunks();
    let vertical_radius = inputs.render_distance.vertical_chunks();

    if inputs.streaming.center != Some(center)
        || inputs.streaming.horizontal_render_distance != horizontal_radius
        || inputs.streaming.vertical_render_distance != vertical_radius
    {
        rebuild_queue(
            &mut inputs.streaming,
            &renderer.pool,
            center,
            horizontal_radius,
            vertical_radius,
            generation.dimension(),
            &content.biomes,
            &content.biome_field,
        );
    }

    let generation_context = generation.context(&content);

    for _ in 0..CHUNKS_PER_FRAME {
        let Some(coord) = inputs.streaming.pending.pop_front() else {
            break;
        };

        if renderer.pool.contains(coord) {
            continue;
        }

        ensure_chunk_loaded(&mut inputs.world, coord, &generation_context);
        fluid_updates.enqueue_loaded_fluid_frontier(&inputs.world, coord);
        let lighting_changes =
            initialize_chunk_lighting(&mut inputs.world, coord, &content.blocks, &content.fluids);
        let chunk = inputs
            .world
            .chunk(coord)
            .unwrap_or_else(|| panic!("generated chunk data should exist at {coord:?}"));
        let render_context = content.render_context(
            &inputs.world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );

        spawn_chunk_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            chunk,
            &render_context,
        );

        for changed in lighting_changes {
            if changed != coord && renderer.pool.contains(changed) {
                inputs.remesh_queue.enqueue_priority(changed);
            }
        }
        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if renderer.pool.contains(neighbor) {
                inputs.remesh_queue.enqueue_priority(neighbor);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    render_pool: &ChunkRenderPool,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) {
    let desired = desired_chunk_coords(
        center,
        horizontal_radius,
        vertical_radius,
        dimension,
        biomes,
        biome_field,
    );
    let mut pending = desired
        .iter()
        .copied()
        .filter(|coord| !render_pool.contains(*coord))
        .collect::<Vec<_>>();

    pending.sort_by_key(|coord| (*coord - center).length_squared());

    streaming.center = Some(center);
    streaming.horizontal_render_distance = horizontal_radius;
    streaming.vertical_render_distance = vertical_radius;
    streaming.desired = desired;
    streaming.pending = pending.into();
}

fn desired_chunk_coords(
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> HashSet<IVec3> {
    let mut desired = chunk_coords_in_volume(center, horizontal_radius, vertical_radius)
        .into_iter()
        .collect::<HashSet<_>>();
    let mut surface_ranges = HashMap::<IVec2, (i32, i32)>::new();

    for z in -horizontal_radius..=horizontal_radius {
        for x in -horizontal_radius..=horizontal_radius {
            if x * x + z * z > horizontal_radius * horizontal_radius {
                continue;
            }

            let horizontal = IVec2::new(center.x + x, center.z + z);
            let (own_minimum, own_maximum) = *surface_ranges.entry(horizontal).or_insert_with(|| {
                chunk_surface_range(horizontal, dimension, biomes, biome_field)
            });
            let mut surrounding_minimum = own_minimum;

            for neighbor_z in -1..=1 {
                for neighbor_x in -1..=1 {
                    if neighbor_x == 0 && neighbor_z == 0 {
                        continue;
                    }

                    let neighbor = horizontal + IVec2::new(neighbor_x, neighbor_z);
                    let (neighbor_minimum, _) = *surface_ranges.entry(neighbor).or_insert_with(|| {
                        chunk_surface_range(neighbor, dimension, biomes, biome_field)
                    });
                    surrounding_minimum = surrounding_minimum.min(neighbor_minimum);
                }
            }

            let chunk_size = CHUNK_SIZE as i32;
            let minimum_y = (surrounding_minimum.div_euclid(chunk_size)
                - SURFACE_PADDING_BELOW_CHUNKS)
                .max(0);
            let maximum_y = (own_maximum.div_euclid(chunk_size)
                + SURFACE_PADDING_ABOVE_CHUNKS)
                .max(minimum_y);

            for y in minimum_y..=maximum_y {
                desired.insert(IVec3::new(horizontal.x, y, horizontal.y));
            }
        }
    }

    desired
}

fn chunk_surface_range(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> (i32, i32) {
    let chunk_size = CHUNK_SIZE as i32;
    let origin = horizontal_chunk * chunk_size;
    let sample_offsets = [0, chunk_size / 2, chunk_size - 1];
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;

    for z in sample_offsets {
        for x in sample_offsets {
            let height = surface_height(origin + IVec2::new(x, z), dimension, biomes, biome_field);
            minimum = minimum.min(height);
            maximum = maximum.max(height);
        }
    }

    (minimum, maximum)
}
