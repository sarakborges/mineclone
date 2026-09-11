use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

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

const MIN_CHUNKS_PER_FRAME: usize = 2;
const MAX_CHUNKS_PER_FRAME: usize = 8;
const EXTRA_STREAMING_BUDGET_MS: u128 = 6;
const HORIZONTAL_PRELOAD_CHUNKS: i32 = 1;
const SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const SURFACE_PADDING_ABOVE_CHUNKS: i32 = 1;
const SURFACE_CACHE_MARGIN_CHUNKS: i32 = 2;

#[derive(Resource, Default)]
pub struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_render_distance: i32,
    vertical_render_distance: i32,
    desired: HashSet<IVec3>,
    pending: VecDeque<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
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
    let frame_started = Instant::now();
    let mut processed = 0;

    while processed < MAX_CHUNKS_PER_FRAME {
        if processed >= MIN_CHUNKS_PER_FRAME
            && frame_started.elapsed().as_millis() >= EXTRA_STREAMING_BUDGET_MS
        {
            break;
        }

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
        processed += 1;

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
    let preload_radius = horizontal_radius + HORIZONTAL_PRELOAD_CHUNKS;
    prune_surface_cache(&mut streaming.surface_ranges, center.xz(), preload_radius);
    let desired = desired_chunk_coords(
        center,
        preload_radius,
        vertical_radius,
        dimension,
        biomes,
        biome_field,
        &mut streaming.surface_ranges,
    );
    let mut pending = desired
        .iter()
        .copied()
        .filter(|coord| !render_pool.contains(*coord))
        .collect::<Vec<_>>();

    pending.sort_by_key(|coord| {
        chunk_streaming_priority(
            *coord,
            center,
            horizontal_radius,
            vertical_radius,
            &streaming.surface_ranges,
        )
    });

    streaming.center = Some(center);
    streaming.horizontal_render_distance = horizontal_radius;
    streaming.vertical_render_distance = vertical_radius;
    streaming.desired = desired;
    streaming.pending = pending.into();
}

fn chunk_streaming_priority(
    coord: IVec3,
    center: IVec3,
    horizontal_render_distance: i32,
    vertical_render_distance: i32,
    surface_ranges: &HashMap<IVec2, (i32, i32)>,
) -> (i32, i32, i32, i32) {
    let horizontal = coord.xz();
    let horizontal_distance_squared = (horizontal - center.xz()).length_squared();
    let vertical_distance = (coord.y - center.y).abs();
    let within_visible_radius = horizontal_distance_squared
        <= horizontal_render_distance * horizontal_render_distance;
    let near_player = horizontal_distance_squared <= 2
        && vertical_distance <= vertical_render_distance;
    let surface_distance = surface_ranges
        .get(&horizontal)
        .map_or(i32::MAX, |(minimum, maximum)| {
            let chunk_size = CHUNK_SIZE as i32;
            let minimum_chunk_y = minimum.div_euclid(chunk_size);
            let maximum_chunk_y = maximum.div_euclid(chunk_size);

            if coord.y < minimum_chunk_y {
                minimum_chunk_y - coord.y
            } else if coord.y > maximum_chunk_y {
                coord.y - maximum_chunk_y
            } else {
                0
            }
        });
    let priority_class = if near_player {
        0
    } else if within_visible_radius && surface_distance <= SURFACE_PADDING_ABOVE_CHUNKS {
        1
    } else if within_visible_radius {
        2
    } else {
        3
    };

    (
        priority_class,
        horizontal_distance_squared,
        surface_distance,
        vertical_distance,
    )
}

fn desired_chunk_coords(
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    surface_ranges: &mut HashMap<IVec2, (i32, i32)>,
) -> HashSet<IVec3> {
    let mut desired = chunk_coords_in_volume(center, horizontal_radius, vertical_radius)
        .into_iter()
        .collect::<HashSet<_>>();

    for z in -horizontal_radius..=horizontal_radius {
        for x in -horizontal_radius..=horizontal_radius {
            if x * x + z * z > horizontal_radius * horizontal_radius {
                continue;
            }

            let horizontal = IVec2::new(center.x + x, center.z + z);
            let (own_minimum, own_maximum) = cached_surface_range(
                surface_ranges,
                horizontal,
                dimension,
                biomes,
                biome_field,
            );
            let mut surrounding_minimum = own_minimum;

            for neighbor_z in -1..=1 {
                for neighbor_x in -1..=1 {
                    if neighbor_x == 0 && neighbor_z == 0 {
                        continue;
                    }

                    let neighbor = horizontal + IVec2::new(neighbor_x, neighbor_z);
                    let (neighbor_minimum, _) = cached_surface_range(
                        surface_ranges,
                        neighbor,
                        dimension,
                        biomes,
                        biome_field,
                    );
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

fn cached_surface_range(
    cache: &mut HashMap<IVec2, (i32, i32)>,
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> (i32, i32) {
    *cache.entry(horizontal_chunk).or_insert_with(|| {
        chunk_surface_range(horizontal_chunk, dimension, biomes, biome_field)
    })
}

fn prune_surface_cache(
    cache: &mut HashMap<IVec2, (i32, i32)>,
    center: IVec2,
    preload_radius: i32,
) {
    let retention_radius = preload_radius + SURFACE_CACHE_MARGIN_CHUNKS;
    let retention_radius_squared = retention_radius * retention_radius;
    cache.retain(|coord, _| (*coord - center).length_squared() <= retention_radius_squared);
}

fn chunk_surface_range(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> (i32, i32) {
    let chunk_size = CHUNK_SIZE as i32;
    let origin = horizontal_chunk * chunk_size;
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;

    for z in 0..chunk_size {
        for x in 0..chunk_size {
            let height = surface_height(origin + IVec2::new(x, z), dimension, biomes, biome_field);
            minimum = minimum.min(height);
            maximum = maximum.max(height);
        }
    }

    (minimum, maximum)
}
