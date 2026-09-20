use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    voxel::chunk::CHUNK_SIZE,
    world::{
        generation::{
            maximum_structure_vertical_chunk_allowance_for_horizontal_chunk,
        },
        generation_region::generation_region_coord,
        render_distance::chunk_is_in_volume,
    },
};

use super::{
    ChunkStreamingState, QueueRebuildContext,
    surface_cache::{cached_surface_range, prune_surface_cache},
};

const HORIZONTAL_PRELOAD_CHUNKS: i32 = 2;
const FORWARD_PRELOAD_CHUNKS: i32 = 8;
const FORWARD_PRELOAD_HALF_WIDTH_CHUNKS: f32 = 8.0;
const SURFACE_PADDING_ABOVE_CHUNKS: i32 = 1;
const NEAR_SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const FAR_SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const PLAYER_LOCAL_VOLUME_RADIUS_CHUNKS: i32 = 3;
const IMMEDIATE_PLAYER_PRIORITY_RADIUS_CHUNKS: i32 = 1;
const SURFACE_SUPPORT_NEIGHBORS: [IVec2; 4] = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];

type PendingPriority = (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32);

#[derive(Clone, Copy)]
struct PendingEntry {
    coord: IVec3,
    priority: PendingPriority,
}

#[derive(Clone, Copy)]
struct DesiredChunkSelection {
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    movement_direction: IVec2,
}

#[derive(Default)]
pub(super) struct QueueRebuildScratch {
    desired: HashSet<IVec3>,
    pending: Vec<PendingEntry>,
    retired: Vec<IVec3>,
    structure_allowances: HashMap<IVec2, i32>,
}

pub(super) fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    scratch: &mut QueueRebuildScratch,
    context: &QueueRebuildContext<'_>,
) {
    update_movement_direction(streaming, center);
    let movement_direction = streaming.movement_direction;
    let prune_caches = should_prune_streaming_caches(
        streaming.center,
        streaming.horizontal_radius,
        streaming.vertical_radius,
        center,
        horizontal_radius,
        vertical_radius,
    );
    let preload_radius = horizontal_radius + HORIZONTAL_PRELOAD_CHUNKS;
    let retention_radius = preload_radius
        + if movement_direction == IVec2::ZERO {
            0
        } else {
            FORWARD_PRELOAD_CHUNKS
        };
    if prune_caches {
        prune_surface_cache(&mut streaming.surface_ranges, center.xz(), retention_radius);
    }

    rebuild_desired_chunk_coords(
        &mut scratch.desired,
        DesiredChunkSelection {
            center,
            horizontal_radius: preload_radius,
            vertical_radius,
            movement_direction,
        },
        context,
        &mut streaming.surface_ranges,
        &mut scratch.structure_allowances,
    );
    if prune_caches {
        context.feature_fields.retain_for_chunks(&scratch.desired);
    }

    let center_structure_allowance = scratch
        .structure_allowances
        .get(&center.xz())
        .copied()
        .unwrap_or(0);
    let prioritize_surface = player_is_above_surface(
        center,
        center_structure_allowance,
        &streaming.surface_ranges,
    );
    scratch.pending.clear();
    let structure_allowances = &scratch.structure_allowances;
    scratch.pending.extend(
        scratch
            .desired
            .iter()
            .copied()
            .filter(|coord| {
                !context.render_pool.contains(*coord)
                    && !streaming.generated_chunk_is_unpublished(*coord)
            })
            .map(|coord| PendingEntry {
                coord,
                priority: pending_priority(
                    coord,
                    center,
                    horizontal_radius,
                    structure_allowances
                        .get(&coord.xz())
                        .copied()
                        .unwrap_or(0),
                    movement_direction,
                    prioritize_surface,
                    &streaming.surface_ranges,
                ),
            }),
    );
    scratch.pending.sort_unstable_by_key(|entry| {
        (
            entry.priority,
            entry.coord.y,
            entry.coord.z,
            entry.coord.x,
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
    streaming.mark_selection_rebuilt();

    streaming.pending.clear();
    streaming.pending.reserve(scratch.pending.len());
    for entry in scratch.pending.drain(..) {
        streaming.pending.enqueue(entry.coord);
    }

    for coord in scratch.retired.drain(..) {
        streaming.enqueue_retired(coord);
    }
}

fn update_movement_direction(streaming: &mut ChunkStreamingState, center: IVec3) {
    let Some(previous_center) = streaming.center else {
        return;
    };
    let delta = center.xz() - previous_center.xz();
    if delta == IVec2::ZERO {
        return;
    }

    streaming.movement_direction = IVec2::new(delta.x.signum(), delta.y.signum());
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

fn player_is_above_surface(
    center: IVec3,
    structure_chunk_allowance: i32,
    surface_ranges: &HashMap<IVec2, (i32, i32)>,
) -> bool {
    let Some((_, maximum_surface)) = surface_ranges.get(&center.xz()).copied() else {
        return false;
    };
    let maximum_structure_chunk =
        maximum_surface.div_euclid(CHUNK_SIZE as i32) + structure_chunk_allowance;
    center.y > maximum_structure_chunk
}

fn pending_priority(
    coord: IVec3,
    center: IVec3,
    visible_radius: i32,
    structure_chunk_allowance: i32,
    movement_direction: IVec2,
    prioritize_surface: bool,
    surface_ranges: &HashMap<IVec2, (i32, i32)>,
) -> PendingPriority {
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
    let horizontal_delta = horizontal - center.xz();
    let horizontal_distance = horizontal_delta.length_squared();
    let vertical_distance = delta.y.abs();
    let total_distance = delta.length_squared();
    let visibility_band = if horizontal_distance <= visible_radius * visible_radius {
        0
    } else {
        1
    };
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
    let player_chunk = if coord == center { 0 } else { 1 };
    let forward = horizontal_delta.dot(movement_direction);
    let directional_band = if movement_direction == IVec2::ZERO || forward == 0 {
        1
    } else if forward > 0 {
        0
    } else {
        2
    };
    let (primary_locality, secondary_locality) = if prioritize_surface {
        (off_surface, immediate_neighborhood)
    } else {
        (immediate_neighborhood, off_surface)
    };

    (
        player_chunk,
        visibility_band,
        primary_locality,
        secondary_locality,
        immediate_distance,
        directional_band,
        horizontal_distance,
        surface_distance,
        vertical_distance,
        total_distance,
    )
}

fn inside_forward_preload(
    offset: IVec2,
    horizontal_radius: i32,
    forward_direction: Vec2,
) -> bool {
    let offset = offset.as_vec2();
    let forward = offset.dot(forward_direction);
    let base = horizontal_radius as f32;
    let limit = base + FORWARD_PRELOAD_CHUNKS as f32;
    if forward <= base || forward > limit {
        return false;
    }

    let extra = forward - base;
    let lateral_width = FORWARD_PRELOAD_HALF_WIDTH_CHUNKS
        + (FORWARD_PRELOAD_CHUNKS as f32 - extra) * 0.5;
    let lateral_squared = (offset.length_squared() - forward * forward).max(0.0);
    lateral_squared <= lateral_width * lateral_width
}

fn rebuild_desired_chunk_coords(
    desired: &mut HashSet<IVec3>,
    selection: DesiredChunkSelection,
    context: &QueueRebuildContext<'_>,
    surface_ranges: &mut HashMap<IVec2, (i32, i32)>,
    structure_allowances: &mut HashMap<IVec2, i32>,
) {
    desired.clear();
    structure_allowances.clear();
    let DesiredChunkSelection {
        center,
        horizontal_radius,
        vertical_radius,
        movement_direction,
    } = selection;

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

    let forward_direction = (movement_direction != IVec2::ZERO)
        .then(|| movement_direction.as_vec2().normalize());
    let search_radius = horizontal_radius
        + if forward_direction.is_some() {
            FORWARD_PRELOAD_CHUNKS
        } else {
            0
        };
    let horizontal_radius_squared = horizontal_radius * horizontal_radius;
    let center_horizontal = center.xz();

    for z in -search_radius..=search_radius {
        for x in -search_radius..=search_radius {
            let offset = IVec2::new(x, z);
            let horizontal_distance_squared = offset.length_squared();
            let inside_base = horizontal_distance_squared <= horizontal_radius_squared;
            let inside_forward = forward_direction
                .is_some_and(|direction| inside_forward_preload(offset, horizontal_radius, direction));
            if !inside_base && !inside_forward {
                continue;
            }

            let horizontal = center_horizontal + offset;
            let (own_minimum, own_maximum) = cached_surface_range(
                surface_ranges,
                horizontal,
                context.dimension,
                context.biomes,
                context.biome_field,
                context.feature_fields,
            );
            let structure_chunk_allowance =
                maximum_structure_vertical_chunk_allowance_for_horizontal_chunk(
                    horizontal,
                    context.biomes,
                    context.structures,
                    context.biome_field,
                    context.feature_fields,
                );
            structure_allowances.insert(horizontal, structure_chunk_allowance);
            let mut surrounding_minimum = own_minimum;

            for neighbor_offset in SURFACE_SUPPORT_NEIGHBORS {
                let neighbor = horizontal + neighbor_offset;
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

    #[test]
    fn pending_entries_use_stable_coordinate_tiebreaker() {
        let priority = (1, 0, 1, 0, 0, 1, 9, 0, 0, 9);
        let mut pending = [
            PendingEntry {
                coord: IVec3::new(3, 0, 0),
                priority,
            },
            PendingEntry {
                coord: IVec3::new(-3, 0, 0),
                priority,
            },
        ];

        pending.sort_unstable_by_key(|entry| {
            (
                entry.priority,
                entry.coord.y,
                entry.coord.z,
                entry.coord.x,
            )
        });

        assert_eq!(pending[0].coord, IVec3::new(-3, 0, 0));
        assert_eq!(pending[1].coord, IVec3::new(3, 0, 0));
    }

    #[test]
    fn movement_direction_prefers_forward_chunks() {
        let surface_ranges = HashMap::from([
            (IVec2::new(3, 0), (0, 0)),
            (IVec2::new(-3, 0), (0, 0)),
        ]);
        let forward = pending_priority(
            IVec3::new(3, 0, 0),
            IVec3::ZERO,
            12,
            0,
            IVec2::X,
            false,
            &surface_ranges,
        );
        let behind = pending_priority(
            IVec3::new(-3, 0, 0),
            IVec3::ZERO,
            12,
            0,
            IVec2::X,
            false,
            &surface_ranges,
        );

        assert!(forward < behind);
    }

    #[test]
    fn visible_chunks_beat_forward_preload_chunks() {
        let center = IVec3::ZERO;
        let visible = IVec3::new(0, 0, 12);
        let preload = IVec3::new(18, 0, 0);
        let surface_ranges = HashMap::from([
            (visible.xz(), (0, 0)),
            (preload.xz(), (0, 0)),
        ]);

        let visible_priority = pending_priority(
            visible,
            center,
            12,
            0,
            IVec2::X,
            false,
            &surface_ranges,
        );
        let preload_priority = pending_priority(
            preload,
            center,
            12,
            0,
            IVec2::X,
            false,
            &surface_ranges,
        );

        assert!(visible_priority < preload_priority);
    }

    #[test]
    fn forward_preload_extends_only_in_front_of_motion() {
        let base_radius = 14;

        assert!(inside_forward_preload(
            IVec2::new(20, 0),
            base_radius,
            Vec2::X
        ));
        assert!(!inside_forward_preload(
            IVec2::new(-20, 0),
            base_radius,
            Vec2::X
        ));
        assert!(!inside_forward_preload(
            IVec2::new(22, 12),
            base_radius,
            Vec2::X
        ));
    }

    #[test]
    fn high_player_prioritizes_surface_before_neighboring_air() {
        let center = IVec3::new(0, 8, 0);
        let surface = IVec3::new(2, 0, 0);
        let air = IVec3::new(1, 8, 0);
        let surface_ranges = HashMap::from([
            (center.xz(), (0, 0)),
            (surface.xz(), (0, 0)),
            (air.xz(), (0, 0)),
        ]);
        assert!(player_is_above_surface(center, 0, &surface_ranges));

        let surface_priority =
            pending_priority(surface, center, 12, 0, IVec2::X, true, &surface_ranges);
        let air_priority =
            pending_priority(air, center, 12, 0, IVec2::X, true, &surface_ranges);

        assert!(surface_priority < air_priority);
    }
}
