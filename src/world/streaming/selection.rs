use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    voxel::{chunk::CHUNK_SIZE, coordinates::chunks_for_block_extent},
    world::{
        generation_region::generation_region_coord,
        render_distance::chunk_is_in_volume,
    },
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
const IMMEDIATE_PLAYER_PRIORITY_RADIUS_CHUNKS: i32 = 1;
const SURFACE_SUPPORT_NEIGHBORS: [IVec2; 4] = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];

#[derive(Default)]
pub(super) struct QueueRebuildScratch {
    desired: HashSet<IVec3>,
    pending: Vec<IVec3>,
    retired: Vec<IVec3>,
}

pub(super) fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    scratch: &mut QueueRebuildScratch,
    context: &QueueRebuildContext<'_>,
) {
    let prune_caches = should_prune_streaming_caches(
        streaming.center,
        streaming.horizontal_radius,
        streaming.vertical_radius,
        center,
        horizontal_radius,
        vertical_radius,
    );
    let horizontal_structure_allowance = chunks_for_block_extent(
        context.structures.max_horizontal_extent_from_anchor(),
    );
    let preload_radius =
        horizontal_radius + HORIZONTAL_PRELOAD_CHUNKS.max(horizontal_structure_allowance);
    let vertical_structure_allowance =
        chunks_for_block_extent(context.structures.max_height_above_anchor());
    if prune_caches {
        prune_surface_cache(&mut streaming.surface_ranges, center.xz(), preload_radius);
    }

    rebuild_desired_chunk_coords(
        &mut scratch.desired,
        center,
        preload_radius,
        vertical_radius,
        vertical_structure_allowance,
        context,
        &mut streaming.surface_ranges,
    );
    if prune_caches {
        context.feature_fields.retain_for_chunks(&scratch.desired);
    }

    scratch.pending.clear();
    scratch.pending.extend(
        scratch
            .desired
            .iter()
            .copied()
            .filter(|coord| !context.render_pool.contains(*coord)),
    );
    scratch.pending.sort_by_cached_key(|coord| {
        pending_priority(
            *coord,
            center,
            vertical_structure_allowance,
            &streaming.surface_ranges,
        )
    });

    collect_retired_chunk_coords(
        &streaming.retained,
        &scratch.desired,
        &streaming.desired,
        center,
        &mut scratch.retired,
    );

    std::mem::swap(&mut streaming.retained, &mut scratch.desired);
    std::mem::swap(&mut streaming.desired, &mut streaming.retained);
    streaming.center = Some(center);
    streaming.horizontal_radius = horizontal_radius;
    streaming.vertical_radius = vertical_radius;

    streaming.pending.clear();
    streaming.pending.reserve(scratch.pending.len());
    for coord in scratch.pending.drain(..) {
        streaming.pending.enqueue(coord);
    }

    for coord in scratch.retired.drain(..) {
        streaming.enqueue_retired(coord);
    }
}

fn should_prune_streaming_caches(
    previous_center: Option<IVec3>,
    previous_horizontal_radius: i32,
    previous_vertical_radius: i32,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
) -> bool {
    previous_center.map(generation_region_coord) != Some(generation_region_coord(center))
        || previous_horizontal_radius != horizontal_radius
        || previous_vertical_radius != vertical_radius
}

fn collect_retired_chunk_coords(
    previous_retained: &HashSet<IVec3>,
    desired: &HashSet<IVec3>,
    retained: &HashSet<IVec3>,
    center: IVec3,
    retired: &mut Vec<IVec3>,
) {
    retired.clear();
    retired.extend(
        previous_retained
            .iter()
            .copied()
            .filter(|coord| !desired.contains(coord) && !retained.contains(coord)),
    );
    retired.sort_by_key(|coord| -(*coord - center).length_squared());
}

fn pending_priority(
    coord: IVec3,
    center: IVec3,
    structure_chunk_allowance: i32,
    surface_ranges: &HashMap<IVec2, (i32, i32)>,
) -> (i32, i32, i32, i32, i32, i32, i32) {
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
    let delta = coord - center;
    let horizontal_distance = (horizontal - center.xz()).length_squared();
    let vertical_distance = delta.y.abs();
    let total_distance = delta.length_squared();
    let off_surface = if surface_distance > 0 { 1 } else { 0 };
    let immediate_neighborhood = if delta.x.abs() <= IMMEDIATE_PLAYER_PRIORITY_RADIUS_CHUNKS
        && delta.y.abs() <= IMMEDIATE_PLAYER_PRIORITY_RADIUS_CHUNKS
        && delta.z.abs() <= IMMEDIATE_PLAYER_PRIORITY_RADIUS_CHUNKS
    {
        0
    } else {
        1
    };
    let immediate_distance = if immediate_neighborhood == 0 {
        total_distance
    } else {
        0
    };

    (
        immediate_neighborhood,
        immediate_distance,
        off_surface,
        horizontal_distance,
        surface_distance,
        vertical_distance,
        total_distance,
    )
}

fn rebuild_desired_chunk_coords(
    desired: &mut HashSet<IVec3>,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    structure_chunk_allowance: i32,
    context: &QueueRebuildContext<'_>,
    surface_ranges: &mut HashMap<IVec2, (i32, i32)>,
) {
    desired.clear();

    let local_radius = horizontal_radius.min(PLAYER_LOCAL_VOLUME_RADIUS_CHUNKS);
    assert!(center.y >= 0, "streaming center Y cannot be negative");
    assert!(
        local_radius >= 0,
        "horizontal streaming radius cannot be negative"
    );
    assert!(
        vertical_radius >= 0,
        "vertical streaming radius cannot be negative"
    );

    let minimum_local_y = (center.y - vertical_radius).max(0);
    let maximum_local_y = center.y + vertical_radius;
    for y in minimum_local_y..=maximum_local_y {
        for z in -local_radius..=local_radius {
            for x in -local_radius..=local_radius {
                let coord = IVec3::new(center.x + x, y, center.z + z);
                if chunk_is_in_volume(center, coord, local_radius, vertical_radius) {
                    desired.insert(coord);
                }
            }
        }
    }

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
                context.dimension,
                context.biomes,
                context.biome_field,
                context.feature_fields,
            );
            let mut surrounding_minimum = own_minimum;

            // Cardinal support is sufficient for the conservative vertical
            // envelope and avoids probing all eight surrounding chunk columns
            // whenever the player crosses into a new streaming center.
            for offset in SURFACE_SUPPORT_NEIGHBORS {
                let neighbor = horizontal + offset;
                let (neighbor_minimum, _) = cached_surface_range(
                    surface_ranges,
                    neighbor,
                    context.dimension,
                    context.biomes,
                    context.biome_field,
                    context.feature_fields,
                );
                surrounding_minimum = surrounding_minimum.min(neighbor_minimum);
            }

            let chunk_size = CHUNK_SIZE as i32;
            let near_player = horizontal_distance_squared <= local_radius * local_radius;
            let padding_below = if near_player {
                NEAR_SURFACE_PADDING_BELOW_CHUNKS
            } else {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_chunks_exclude_both_live_selection_generations() {
        let far = IVec3::new(4, 0, 0);
        let nearer = IVec3::new(3, 0, 0);
        let still_desired = IVec3::new(2, 0, 0);
        let still_retained = IVec3::new(1, 0, 0);
        let previous_retained = HashSet::from([far, nearer, still_desired, still_retained]);
        let desired = HashSet::from([still_desired]);
        let retained = HashSet::from([still_retained]);
        let mut retired = Vec::new();

        collect_retired_chunk_coords(
            &previous_retained,
            &desired,
            &retained,
            IVec3::ZERO,
            &mut retired,
        );

        assert_eq!(retired, vec![far, nearer]);
    }

    #[test]
    fn streaming_cache_pruning_follows_generation_region_boundaries() {
        let radius = 12;
        let vertical_radius = 4;

        assert!(should_prune_streaming_caches(
            None,
            radius,
            vertical_radius,
            IVec3::ZERO,
            radius,
            vertical_radius,
        ));
        assert!(!should_prune_streaming_caches(
            Some(IVec3::ZERO),
            radius,
            vertical_radius,
            IVec3::new(7, 0, 7),
            radius,
            vertical_radius,
        ));
        assert!(should_prune_streaming_caches(
            Some(IVec3::new(7, 0, 7)),
            radius,
            vertical_radius,
            IVec3::new(8, 0, 7),
            radius,
            vertical_radius,
        ));
        assert!(should_prune_streaming_caches(
            Some(IVec3::ZERO),
            radius,
            vertical_radius,
            IVec3::ZERO,
            radius + 1,
            vertical_radius,
        ));
    }
}
