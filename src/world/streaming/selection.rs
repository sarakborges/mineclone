use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, dimension::DimensionDefinition, structure::StructureRegistry,
    },
    voxel::{chunk::CHUNK_SIZE, coordinates::chunks_for_block_extent},
    world::{biome_field::BiomeField, render_distance::chunk_coords_in_volume},
};

use super::{
    ChunkStreamingState, QueueRebuildContext,
    surface_cache::{cached_surface_range, prune_surface_cache},
};

const HORIZONTAL_PRELOAD_CHUNKS: i32 = 1;
const SURFACE_PADDING_ABOVE_CHUNKS: i32 = 1;
const NEAR_SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const FAR_SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const PLAYER_LOCAL_VOLUME_RADIUS_CHUNKS: i32 = 3;

pub(super) fn initial_chunk_coords(
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    structures: &StructureRegistry,
    biome_field: &BiomeField,
) -> Vec<IVec3> {
    let horizontal_structure_allowance =
        chunks_for_block_extent(structures.max_horizontal_extent_from_anchor());
    let preload_radius =
        horizontal_radius + HORIZONTAL_PRELOAD_CHUNKS.max(horizontal_structure_allowance);
    let vertical_structure_allowance =
        chunks_for_block_extent(structures.max_height_above_anchor());
    let mut surface_ranges = HashMap::new();
    let desired = desired_chunk_coords(
        center,
        preload_radius,
        vertical_radius,
        vertical_structure_allowance,
        dimension,
        biomes,
        biome_field,
        &mut surface_ranges,
    );
    let mut coords = desired.into_iter().collect::<Vec<_>>();

    coords.sort_by_key(|coord| {
        pending_priority(
            *coord,
            center,
            vertical_structure_allowance,
            &surface_ranges,
        )
    });
    coords
}

pub(super) fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    context: &QueueRebuildContext<'_>,
) {
    let horizontal_structure_allowance = chunks_for_block_extent(
        context.structures.max_horizontal_extent_from_anchor(),
    );
    let preload_radius =
        horizontal_radius + HORIZONTAL_PRELOAD_CHUNKS.max(horizontal_structure_allowance);
    let vertical_structure_allowance =
        chunks_for_block_extent(context.structures.max_height_above_anchor());
    prune_surface_cache(&mut streaming.surface_ranges, center.xz(), preload_radius);
    let desired = desired_chunk_coords(
        center,
        preload_radius,
        vertical_radius,
        vertical_structure_allowance,
        context.dimension,
        context.biomes,
        context.biome_field,
        &mut streaming.surface_ranges,
    );
    context.feature_fields.retain_for_chunks(&desired);
    let mut pending = desired
        .iter()
        .copied()
        .filter(|coord| !context.render_pool.contains(*coord))
        .collect::<Vec<_>>();

    pending.sort_by_key(|coord| {
        pending_priority(
            *coord,
            center,
            vertical_structure_allowance,
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
) -> (i32, i32, i32, i32, i32) {
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
    let off_surface = if surface_distance > 0 { 1 } else { 0 };

    (
        off_surface,
        horizontal_distance,
        surface_distance,
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
    let local_radius = horizontal_radius.min(PLAYER_LOCAL_VOLUME_RADIUS_CHUNKS);
    let mut desired = chunk_coords_in_volume(center, local_radius, vertical_radius)
        .into_iter()
        .collect::<HashSet<_>>();

    for z in -horizontal_radius..=horizontal_radius {
        for x in -horizontal_radius..=horizontal_radius {
            let horizontal_distance_squared = x * x + z * z;
            if horizontal_distance_squared > horizontal_radius * horizontal_radius {
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
            let near_player = horizontal_distance_squared <= local_radius * local_radius;
            let padding_below = if near_player {
                NEAR_SURFACE_PADDING_BELOW_CHUNKS
            } else {
                // Lakes can carve eighteen blocks below their source surface,
                // already deeper than one chunk. Two support chunks prevent
                // distant hydrology cuts from opening directly into unloaded void.
                FAR_SURFACE_PADDING_BELOW_CHUNKS
            };
            let minimum_y =
                (surrounding_minimum.div_euclid(chunk_size) - padding_below).max(0);
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
