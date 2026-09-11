use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        biome::BiomeRegistry, dimension::DimensionDefinition, structure::StructureRegistry,
    },
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        chunk::CHUNK_SIZE, coordinates::split_dimension_position,
        lighting::initialize_chunks_lighting, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
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

const MIN_CHUNKS_BEFORE_BUDGET_CHECK: usize = 2;
const STREAMING_LIGHT_BATCH_CHUNKS: usize = 2;
const STREAMING_BUDGET_MS: u128 = 6;
const HORIZONTAL_PRELOAD_CHUNKS: i32 = 1;
const SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const SURFACE_PADDING_ABOVE_CHUNKS: i32 = 1;
const SURFACE_CACHE_MARGIN_CHUNKS: i32 = 2;

#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_radius: i32,
    vertical_radius: i32,
    desired: HashSet<IVec3>,
    pending: VecDeque<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
}

impl ChunkStreamingState {
    pub(super) fn wants(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord)
    }
}

struct QueueRebuildContext<'a> {
    render_pool: &'a ChunkRenderPool,
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
    structures: &'a StructureRegistry,
    biome_field: &'a BiomeField,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingInputs<'w, 's> {
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    render_distance: Res<'w, RenderDistanceSettings>,
    world: ResMut<'w, VoxelWorld>,
    streaming: ResMut<'w, ChunkStreamingState>,
    remesh_queue: ResMut<'w, ChunkRemeshQueue>,
}

pub(super) fn reset_chunk_streaming(mut state: ResMut<ChunkStreamingState>) {
    *state = ChunkStreamingState::default();
}

pub(super) fn stream_chunks(
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
        || inputs.streaming.horizontal_radius != horizontal_radius
        || inputs.streaming.vertical_radius != vertical_radius
    {
        let rebuild_context = QueueRebuildContext {
            render_pool: &renderer.pool,
            dimension: generation.dimension(),
            biomes: &content.biomes,
            structures: &content.structures,
            biome_field: &content.biome_field,
        };
        rebuild_queue(
            &mut inputs.streaming,
            center,
            horizontal_radius,
            vertical_radius,
            &rebuild_context,
        );
    }

    let generation_context = generation.context(&content);
    let frame_started = Instant::now();
    let mut processed = 0;

    loop {
        if processed >= MIN_CHUNKS_BEFORE_BUDGET_CHECK
            && frame_started.elapsed().as_millis() >= STREAMING_BUDGET_MS
        {
            break;
        }

        let mut batch = Vec::with_capacity(STREAMING_LIGHT_BATCH_CHUNKS);

        while batch.len() < STREAMING_LIGHT_BATCH_CHUNKS {
            let Some(coord) = inputs.streaming.pending.pop_front() else {
                break;
            };

            if renderer.pool.contains(coord) {
                continue;
            }

            ensure_chunk_loaded(&mut inputs.world, coord, &generation_context);
            fluid_updates.enqueue_loaded_fluid_frontier(&inputs.world, coord);
            batch.push(coord);
        }

        if batch.is_empty() {
            break;
        }

        let lighting_changes = initialize_chunks_lighting(
            &mut inputs.world,
            &batch,
            &content.blocks,
            &content.fluids,
        );

        for &coord in &batch {
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
        }

        processed += batch.len();

        for changed in lighting_changes {
            if !batch.contains(&changed) && renderer.pool.contains(changed) {
                inputs.remesh_queue.enqueue_priority(changed);
            }
        }
        for &coord in &batch {
            for offset in CARDINAL_NEIGHBORS {
                let neighbor = coord + offset;
                if !batch.contains(&neighbor) && renderer.pool.contains(neighbor) {
                    inputs.remesh_queue.enqueue_priority(neighbor);
                }
            }
        }
    }
}

fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    context: &QueueRebuildContext<'_>,
) {
    let preload_radius = horizontal_radius + HORIZONTAL_PRELOAD_CHUNKS;
    let structure_chunk_allowance = structure_chunk_allowance(context.structures);
    prune_surface_cache(&mut streaming.surface_ranges, center.xz(), preload_radius);
    let desired = desired_chunk_coords(
        center,
        preload_radius,
        vertical_radius,
        structure_chunk_allowance,
        context.dimension,
        context.biomes,
        context.biome_field,
        &mut streaming.surface_ranges,
    );
    let mut pending = desired
        .iter()
        .copied()
        .filter(|coord| !context.render_pool.contains(*coord))
        .collect::<Vec<_>>();

    pending.sort_by_key(|coord| {
        pending_priority(
            *coord,
            center,
            structure_chunk_allowance,
            &streaming.surface_ranges,
        )
    });

    streaming.center = Some(center);
    streaming.horizontal_radius = horizontal_radius;
    streaming.vertical_radius = vertical_radius;
    streaming.desired = desired;
    streaming.pending = pending.into();
}

fn pending_priority(
    coord: IVec3,
    center: IVec3,
    structure_chunk_allowance: i32,
    surface_ranges: &HashMap<IVec2, (i32, i32)>,
) -> (i32, i32, i32, i32) {
    let chunk_size = CHUNK_SIZE as i32;
    let horizontal = coord.xz();
    let (minimum_surface, maximum_surface) = surface_ranges
        .get(&horizontal)
        .copied()
        .unwrap_or((coord.y * chunk_size, coord.y * chunk_size));
    let minimum_surface_chunk = minimum_surface.div_euclid(chunk_size);
    let maximum_structure_chunk =
        maximum_surface.div_euclid(chunk_size) + structure_chunk_allowance;
    let surface_distance = if coord.y < minimum_surface_chunk {
        minimum_surface_chunk - coord.y
    } else if coord.y > maximum_structure_chunk {
        coord.y - maximum_structure_chunk
    } else {
        0
    };
    let horizontal_distance = (horizontal - center.xz()).length_squared();
    let vertical_distance = (coord.y - center.y).abs();
    let total_distance = (coord - center).length_squared();

    (
        surface_distance,
        horizontal_distance,
        vertical_distance,
        total_distance,
    )
}

fn desired_chunk_coords(
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    structure_chunk_allowance: i32,
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
                + SURFACE_PADDING_ABOVE_CHUNKS.max(structure_chunk_allowance))
                .max(minimum_y);

            for y in minimum_y..=maximum_y {
                desired.insert(IVec3::new(horizontal.x, y, horizontal.y));
            }
        }
    }

    desired
}

fn structure_chunk_allowance(structures: &StructureRegistry) -> i32 {
    let structure_height = structures.max_height_above_anchor();
    if structure_height == 0 {
        0
    } else {
        (structure_height + CHUNK_SIZE as i32 - 1) / CHUNK_SIZE as i32
    }
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
