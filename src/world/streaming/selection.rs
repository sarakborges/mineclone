use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::CHUNK_SIZE,
    world::{
        biome_field::BiomeField,
        generation_region::generation_region_coord,
        render_distance::chunk_is_in_volume,
        world_feature_fields::WorldFeatureFields,
    },
};

use super::{
    ChunkStreamingState, QueueRebuildContext,
    surface_cache::{cached_surface_range, prune_surface_cache},
};

const HORIZONTAL_PRELOAD_CHUNKS: i32 = 2;
const MAX_FORWARD_PRELOAD_CHUNKS: i32 = 8;
const FORWARD_PRELOAD_HALF_WIDTH_CHUNKS: f32 = 8.0;
const NEAR_SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const FAR_SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const PLAYER_LOCAL_VOLUME_RADIUS_CHUNKS: i32 = 3;
const IMMEDIATE_PLAYER_PRIORITY_RADIUS_CHUNKS: i32 = 1;
const SURFACE_SUPPORT_NEIGHBORS: [IVec2; 4] = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];

type PendingPriority = (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32);

#[derive(Clone, Copy)]
struct DesiredChunkSelection {
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
}

#[derive(Clone, Copy)]
struct SurfaceSelectionContext<'a> {
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
    biome_field: &'a BiomeField,
    feature_fields: &'a WorldFeatureFields,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HorizontalSelectionShapeKey {
    radius: i32,
    movement_direction: IVec2,
}

#[derive(Default)]
pub(super) struct QueueRebuildScratch {
    desired: HashSet<IVec3>,
    retained: HashSet<IVec3>,
    pending: Vec<IVec3>,
    retired: Vec<IVec3>,
    newly_desired: Vec<IVec3>,
    no_longer_desired: Vec<IVec3>,
    horizontal_shape_key: Option<HorizontalSelectionShapeKey>,
    horizontal_offsets: Vec<IVec2>,
}

pub(in crate::world) fn initial_streaming_chunk_coords(
    center: IVec3,
    render_distance_chunks: i32,
    vertical_radius: i32,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
) -> Vec<IVec3> {
    let horizontal_radius = render_distance_chunks + HORIZONTAL_PRELOAD_CHUNKS;
    let selection = DesiredChunkSelection { center, horizontal_radius, vertical_radius };
    let context = SurfaceSelectionContext { dimension, biomes, biome_field, feature_fields };
    let mut horizontal_offsets = Vec::new();
    rebuild_horizontal_selection_offsets(&mut horizontal_offsets, horizontal_radius, IVec2::ZERO);

    let mut desired = HashSet::new();
    let mut surface_ranges = HashMap::new();
    let mut surface_support_minimums = HashMap::new();
    let mut structure_top_chunks = HashMap::new();
    rebuild_desired_chunk_coords(
        &mut desired,
        selection,
        &horizontal_offsets,
        context,
        &mut surface_ranges,
        &mut surface_support_minimums,
        &mut structure_top_chunks,
    );
    let mut coords = desired.into_iter().collect::<Vec<_>>();
    coords.sort_by_key(|coord| (*coord - center).length_squared());
    coords
}

pub(super) fn rebuild_queue(
    streaming: &mut ChunkStreamingState,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    allow_forward_preload: bool,
    scratch: &mut QueueRebuildScratch,
    context: &QueueRebuildContext<'_>,
) {
    let previous_center = streaming.center;
    let previous_movement_direction = streaming.movement_direction;
    let previous_horizontal_radius = streaming.horizontal_radius;
    let previous_vertical_radius = streaming.vertical_radius;

    if allow_forward_preload {
        update_movement_direction(streaming, center);
    } else {
        streaming.movement_direction = IVec2::ZERO;
    }
    let movement_direction = streaming.movement_direction;
    let prune_caches = should_prune_streaming_caches(
        previous_center,
        previous_horizontal_radius,
        previous_vertical_radius,
        center,
        horizontal_radius,
        vertical_radius,
    );
    let preload_radius = horizontal_radius + HORIZONTAL_PRELOAD_CHUNKS;
    let forward_preload = forward_preload_chunks(preload_radius);
    let retention_radius = preload_radius
        + if movement_direction == IVec2::ZERO {
            0
        } else {
            forward_preload
        };
    if prune_caches {
        prune_surface_cache(&mut streaming.surface_ranges, center.xz(), retention_radius);
        let retention_radius_squared = retention_radius * retention_radius;
        streaming.surface_support_minimums.retain(|coord, _| {
            (*coord - center.xz()).length_squared() <= retention_radius_squared
        });
        streaming.structure_top_chunks.retain(|coord, _| {
            (*coord - center.xz()).length_squared() <= retention_radius_squared
        });
    }

    sync_horizontal_selection_offsets(
        scratch,
        preload_radius,
        movement_direction,
    );
    let desired_selection = DesiredChunkSelection {
        center,
        horizontal_radius: preload_radius,
        vertical_radius,
    };
    let surface_context = SurfaceSelectionContext {
        dimension: context.dimension,
        biomes: context.biomes,
        biome_field: context.biome_field,
        feature_fields: context.feature_fields,
    };
    let incremental_rebuild = can_incrementally_rebuild_desired(
        previous_center,
        previous_horizontal_radius,
        previous_vertical_radius,
        previous_movement_direction,
        center,
        horizontal_radius,
        vertical_radius,
        movement_direction,
        allow_forward_preload,
        prune_caches,
    );
    if incremental_rebuild {
        rebuild_desired_chunk_coords_incremental(
            &mut scratch.desired,
            &streaming.desired,
            previous_center.expect("incremental rebuild requires previous center"),
            desired_selection,
            movement_direction,
            &scratch.horizontal_offsets,
            surface_context,
            &mut streaming.surface_ranges,
            &mut streaming.surface_support_minimums,
            &mut streaming.structure_top_chunks,
            &mut scratch.newly_desired,
            &mut scratch.no_longer_desired,
        );
    } else {
        scratch.newly_desired.clear();
        scratch.no_longer_desired.clear();
        rebuild_desired_chunk_coords(
            &mut scratch.desired,
            desired_selection,
            &scratch.horizontal_offsets,
            surface_context,
            &mut streaming.surface_ranges,
            &mut streaming.surface_support_minimums,
            &mut streaming.structure_top_chunks,
        );
        scratch.no_longer_desired.extend(
            streaming
                .desired
                .difference(&scratch.desired)
                .copied(),
        );
    }
    if prune_caches {
        context.feature_fields.retain_for_chunks(&scratch.desired);
        context
            .biome_field
            .retain_surface_site_cache(center.xz(), retention_radius);
    }

    streaming.retain_mesh_pressure_evictions(&scratch.desired, center);

    if incremental_rebuild {
        apply_incremental_pending_delta(
            streaming,
            context.render_pool,
            &scratch.newly_desired,
            &scratch.no_longer_desired,
        );
    } else {
        scratch.pending.clear();
        scratch.pending.extend(
            scratch
                .desired
                .iter()
                .copied()
                .filter(|coord| {
                    !context.render_pool.contains(*coord)
                        && !streaming.generated_chunk_is_unpublished(*coord)
                        && !streaming.mesh_is_pressure_evicted(*coord)
                }),
        );
    }

    scratch.retained.clear();
    scratch
        .retained
        .extend(scratch.no_longer_desired.iter().copied());

    collect_retired_chunk_coords(
        &streaming.retained,
        &scratch.desired,
        &scratch.retained,
        center,
        &mut scratch.retired,
    );

    std::mem::swap(&mut streaming.desired, &mut scratch.desired);
    std::mem::swap(&mut streaming.retained, &mut scratch.retained);
    streaming.center = Some(center);
    streaming.horizontal_radius = horizontal_radius;
    streaming.vertical_radius = vertical_radius;
    streaming.mark_selection_rebuilt();

    if !incremental_rebuild {
        streaming.pending.clear();
        streaming.pending.reserve(scratch.pending.len());
        for coord in scratch.pending.drain(..) {
            streaming.pending.enqueue(coord);
        }
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

#[allow(clippy::too_many_arguments)]
fn can_incrementally_rebuild_desired(
    previous_center: Option<IVec3>,
    previous_horizontal_radius: i32,
    previous_vertical_radius: i32,
    previous_movement_direction: IVec2,
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
    movement_direction: IVec2,
    allow_forward_preload: bool,
    prune_caches: bool,
) -> bool {
    let Some(previous_center) = previous_center else {
        return false;
    };
    let delta = center - previous_center;

    allow_forward_preload
        && !prune_caches
        && previous_horizontal_radius == horizontal_radius
        && previous_vertical_radius == vertical_radius
        && previous_movement_direction == movement_direction
        && delta.y == 0
        && delta.x.abs() <= 1
        && delta.z.abs() <= 1
        && delta.xz() != IVec2::ZERO
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

pub(super) fn player_is_above_surface(
    center: IVec3,
    structure_top_chunk: i32,
    surface_ranges: &HashMap<IVec2, (i32, i32)>,
) -> bool {
    let Some((_, maximum_surface)) = surface_ranges.get(&center.xz()).copied() else {
        return false;
    };
    let maximum_structure_chunk =
        maximum_surface.div_euclid(CHUNK_SIZE as i32).max(structure_top_chunk);
    center.y > maximum_structure_chunk
}

pub(super) fn pending_priority(
    coord: IVec3,
    center: IVec3,
    visible_radius: i32,
    structure_top_chunk: i32,
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
        maximum_surface.div_euclid(chunk_size).max(structure_top_chunk);
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
        horizontal_distance,
        total_distance,
        vertical_distance,
        primary_locality,
        secondary_locality,
        surface_distance,
        directional_band,
        visibility_band,
        player_chunk,
        immediate_distance,
    )
}

fn forward_preload_chunks(horizontal_radius: i32) -> i32 {
    (horizontal_radius / 2).clamp(2, MAX_FORWARD_PRELOAD_CHUNKS)
}

fn inside_forward_preload(
    offset: IVec2,
    horizontal_radius: i32,
    forward_direction: Vec2,
) -> bool {
    let offset = offset.as_vec2();
    let forward = offset.dot(forward_direction);
    let base = horizontal_radius as f32;
    let preload = forward_preload_chunks(horizontal_radius) as f32;
    let limit = base + preload;
    if forward <= base || forward > limit {
        return false;
    }

    let extra = forward - base;
    let lateral_width =
        FORWARD_PRELOAD_HALF_WIDTH_CHUNKS + (preload - extra) * 0.5;
    let lateral_squared = (offset.length_squared() - forward * forward).max(0.0);
    lateral_squared <= lateral_width * lateral_width
}

fn horizontal_offset_is_selected(
    offset: IVec2,
    horizontal_radius: i32,
    movement_direction: IVec2,
) -> bool {
    if offset.length_squared() <= horizontal_radius * horizontal_radius {
        return true;
    }
    movement_direction != IVec2::ZERO
        && inside_forward_preload(
            offset,
            horizontal_radius,
            movement_direction.as_vec2().normalize(),
        )
}

fn sync_horizontal_selection_offsets(
    scratch: &mut QueueRebuildScratch,
    horizontal_radius: i32,
    movement_direction: IVec2,
) {
    let key = HorizontalSelectionShapeKey {
        radius: horizontal_radius,
        movement_direction,
    };
    if scratch.horizontal_shape_key == Some(key) {
        return;
    }

    scratch.horizontal_offsets.clear();
    rebuild_horizontal_selection_offsets(
        &mut scratch.horizontal_offsets,
        horizontal_radius,
        movement_direction,
    );
    scratch.horizontal_shape_key = Some(key);
}

fn rebuild_horizontal_selection_offsets(
    offsets: &mut Vec<IVec2>,
    horizontal_radius: i32,
    movement_direction: IVec2,
) {
    offsets.clear();
    let radius_squared = horizontal_radius * horizontal_radius;
    for z in -horizontal_radius..=horizontal_radius {
        for x in -horizontal_radius..=horizontal_radius {
            let offset = IVec2::new(x, z);
            if offset.length_squared() <= radius_squared {
                offsets.push(offset);
            }
        }
    }

    if movement_direction == IVec2::ZERO {
        return;
    }

    let forward = movement_direction.as_vec2().normalize();
    let lateral = Vec2::new(-forward.y, forward.x);
    let preload = forward_preload_chunks(horizontal_radius) as f32;
    let base = horizontal_radius as f32;
    let limit = base + preload;
    let near_width = FORWARD_PRELOAD_HALF_WIDTH_CHUNKS + preload * 0.5;
    let far_width = FORWARD_PRELOAD_HALF_WIDTH_CHUNKS;
    let corners = [
        forward * base + lateral * near_width,
        forward * base - lateral * near_width,
        forward * limit + lateral * far_width,
        forward * limit - lateral * far_width,
    ];
    let minimum = corners
        .iter()
        .copied()
        .reduce(Vec2::min)
        .expect("forward preload bounds require corners")
        .floor()
        .as_ivec2()
        - IVec2::ONE;
    let maximum = corners
        .iter()
        .copied()
        .reduce(Vec2::max)
        .expect("forward preload bounds require corners")
        .ceil()
        .as_ivec2()
        + IVec2::ONE;

    for z in minimum.y..=maximum.y {
        for x in minimum.x..=maximum.x {
            let offset = IVec2::new(x, z);
            if offset.length_squared() <= radius_squared {
                continue;
            }
            if inside_forward_preload(offset, horizontal_radius, forward) {
                offsets.push(offset);
            }
        }
    }
}

fn rebuild_desired_chunk_coords(
    desired: &mut HashSet<IVec3>,
    selection: DesiredChunkSelection,
    horizontal_offsets: &[IVec2],
    context: SurfaceSelectionContext<'_>,
    surface_ranges: &mut HashMap<IVec2, (i32, i32)>,
    surface_support_minimums: &mut HashMap<IVec2, i32>,
    structure_top_chunks: &mut HashMap<IVec2, i32>,
) {
    desired.clear();
    insert_local_volume(desired, selection);

    let local_radius = selection
        .horizontal_radius
        .min(PLAYER_LOCAL_VOLUME_RADIUS_CHUNKS);
    let center_horizontal = selection.center.xz();
    for &offset in horizontal_offsets {
        insert_surface_column(
            desired,
            center_horizontal + offset,
            offset.length_squared(),
            local_radius,
            context,
            surface_ranges,
            surface_support_minimums,
            structure_top_chunks,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_desired_chunk_coords_incremental(
    desired: &mut HashSet<IVec3>,
    previous_desired: &HashSet<IVec3>,
    previous_center: IVec3,
    selection: DesiredChunkSelection,
    movement_direction: IVec2,
    horizontal_offsets: &[IVec2],
    context: SurfaceSelectionContext<'_>,
    surface_ranges: &mut HashMap<IVec2, (i32, i32)>,
    surface_support_minimums: &mut HashMap<IVec2, i32>,
    structure_top_chunks: &mut HashMap<IVec2, i32>,
    newly_desired: &mut Vec<IVec3>,
    no_longer_desired: &mut Vec<IVec3>,
) {
    desired.clear();
    newly_desired.clear();
    no_longer_desired.clear();
    let local_radius = selection
        .horizontal_radius
        .min(PLAYER_LOCAL_VOLUME_RADIUS_CHUNKS);

    for &coord in previous_desired {
        if coord_remains_selected(
            coord,
            selection,
            movement_direction,
            local_radius,
            surface_ranges,
            surface_support_minimums,
            structure_top_chunks,
        ) {
            desired.insert(coord);
        } else {
            no_longer_desired.push(coord);
        }
    }
    insert_local_volume(desired, selection);

    let center_horizontal = selection.center.xz();
    let previous_horizontal = previous_center.xz();
    for &offset in horizontal_offsets {
        let horizontal = center_horizontal + offset;
        let previous_offset = horizontal - previous_horizontal;
        let was_selected =
            horizontal_offset_is_selected(previous_offset, selection.horizontal_radius, movement_direction);
        let has_cached_range = surface_ranges.contains_key(&horizontal)
            && surface_support_minimums.contains_key(&horizontal);

        if was_selected && has_cached_range {
            continue;
        }

        insert_surface_column(
            desired,
            horizontal,
            offset.length_squared(),
            local_radius,
            context,
            surface_ranges,
            surface_support_minimums,
            structure_top_chunks,
        );
    }

    no_longer_desired.retain(|coord| !desired.contains(coord));
    newly_desired.extend(desired.difference(previous_desired).copied());
}

fn apply_incremental_pending_delta(
    streaming: &mut ChunkStreamingState,
    render_pool: &crate::world::chunk_rendering::ChunkRenderPool,
    newly_desired: &[IVec3],
    no_longer_desired: &[IVec3],
) {
    for &coord in no_longer_desired {
        streaming.pending.remove(coord);
    }

    for &coord in newly_desired {
        if !render_pool.contains(coord)
            && !streaming.generated_chunk_is_unpublished(coord)
            && !streaming.mesh_is_pressure_evicted(coord)
        {
            streaming.pending.enqueue(coord);
        }
    }
}

fn insert_local_volume(desired: &mut HashSet<IVec3>, selection: DesiredChunkSelection) {
    let DesiredChunkSelection {
        center,
        horizontal_radius,
        vertical_radius,
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
}

#[allow(clippy::too_many_arguments)]
fn insert_surface_column(
    desired: &mut HashSet<IVec3>,
    horizontal: IVec2,
    horizontal_distance_squared: i32,
    local_radius: i32,
    context: SurfaceSelectionContext<'_>,
    surface_ranges: &mut HashMap<IVec2, (i32, i32)>,
    surface_support_minimums: &mut HashMap<IVec2, i32>,
    structure_top_chunks: &HashMap<IVec2, i32>,
) {
    let (own_minimum, own_maximum) = cached_surface_range(
        surface_ranges,
        horizontal,
        context.dimension,
        context.biomes,
        context.biome_field,
        context.feature_fields,
    );
    let surrounding_minimum = *surface_support_minimums
        .entry(horizontal)
        .or_insert_with(|| {
            let mut minimum = own_minimum;
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
                minimum = minimum.min(neighbor_minimum);
            }
            minimum
        });
    let structure_top_chunk = structure_top_chunks
        .get(&horizontal)
        .copied()
        .unwrap_or(-1);
    let (minimum_y, maximum_y) = surface_chunk_range(
        own_maximum,
        surrounding_minimum,
        structure_top_chunk,
        horizontal_distance_squared <= local_radius * local_radius,
    );

    for y in minimum_y..=maximum_y {
        desired.insert(IVec3::new(horizontal.x, y, horizontal.y));
    }
}

fn coord_remains_selected(
    coord: IVec3,
    selection: DesiredChunkSelection,
    movement_direction: IVec2,
    local_radius: i32,
    surface_ranges: &HashMap<IVec2, (i32, i32)>,
    surface_support_minimums: &HashMap<IVec2, i32>,
    structure_top_chunks: &HashMap<IVec2, i32>,
) -> bool {
    if chunk_is_in_volume(
        selection.center,
        coord,
        local_radius,
        selection.vertical_radius,
    ) {
        return true;
    }

    let horizontal = coord.xz();
    let offset = horizontal - selection.center.xz();
    if !horizontal_offset_is_selected(
        offset,
        selection.horizontal_radius,
        movement_direction,
    ) {
        return false;
    }

    let Some((_, own_maximum)) = surface_ranges.get(&horizontal).copied() else {
        return false;
    };
    let Some(surrounding_minimum) = surface_support_minimums.get(&horizontal).copied() else {
        return false;
    };
    let structure_top_chunk = structure_top_chunks
        .get(&horizontal)
        .copied()
        .unwrap_or(-1);
    let (minimum_y, maximum_y) = surface_chunk_range(
        own_maximum,
        surrounding_minimum,
        structure_top_chunk,
        offset.length_squared() <= local_radius * local_radius,
    );
    (minimum_y..=maximum_y).contains(&coord.y)
}

fn surface_chunk_range(
    own_maximum: i32,
    surrounding_minimum: i32,
    structure_top_chunk: i32,
    near_player: bool,
) -> (i32, i32) {
    let chunk_size = CHUNK_SIZE as i32;
    let padding_below = if near_player {
        NEAR_SURFACE_PADDING_BELOW_CHUNKS
    } else {
        FAR_SURFACE_PADDING_BELOW_CHUNKS
    };
    let minimum_y = (surrounding_minimum.div_euclid(chunk_size) - padding_below).max(0);
    let maximum_y = own_maximum
        .div_euclid(chunk_size)
        .max(structure_top_chunk)
        .max(minimum_y);
    (minimum_y, maximum_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute_force_horizontal_offsets(
        horizontal_radius: i32,
        movement_direction: IVec2,
    ) -> HashSet<IVec2> {
        let forward_direction = (movement_direction != IVec2::ZERO)
            .then(|| movement_direction.as_vec2().normalize());
        let search_radius = horizontal_radius
            + if forward_direction.is_some() {
                forward_preload_chunks(horizontal_radius)
            } else {
                0
            };
        let radius_squared = horizontal_radius * horizontal_radius;
        let mut offsets = HashSet::new();

        for z in -search_radius..=search_radius {
            for x in -search_radius..=search_radius {
                let offset = IVec2::new(x, z);
                let inside_base = offset.length_squared() <= radius_squared;
                let inside_forward = forward_direction.is_some_and(|direction| {
                    inside_forward_preload(offset, horizontal_radius, direction)
                });
                if inside_base || inside_forward {
                    offsets.insert(offset);
                }
            }
        }

        offsets
    }

    #[test]
    fn incremental_rebuild_only_handles_adjacent_same_direction_motion() {
        let previous = IVec3::new(10, 2, 10);

        assert!(can_incrementally_rebuild_desired(
            Some(previous),
            12,
            2,
            IVec2::X,
            previous + IVec3::X,
            12,
            2,
            IVec2::X,
            true,
            false,
        ));
        assert!(!can_incrementally_rebuild_desired(
            Some(previous),
            12,
            2,
            IVec2::X,
            previous + IVec3::new(2, 0, 0),
            12,
            2,
            IVec2::X,
            true,
            false,
        ));
        assert!(!can_incrementally_rebuild_desired(
            Some(previous),
            12,
            2,
            IVec2::X,
            previous + IVec3::X,
            12,
            2,
            IVec2::Y,
            true,
            false,
        ));
        assert!(!can_incrementally_rebuild_desired(
            Some(previous),
            12,
            2,
            IVec2::X,
            previous + IVec3::X,
            12,
            2,
            IVec2::X,
            true,
            true,
        ));
    }

    #[test]
    fn optimized_horizontal_selection_matches_previous_bruteforce_shape() {
        let directions = [
            IVec2::ZERO,
            IVec2::X,
            IVec2::NEG_X,
            IVec2::Y,
            IVec2::NEG_Y,
            IVec2::new(1, 1),
            IVec2::new(1, -1),
            IVec2::new(-1, 1),
            IVec2::new(-1, -1),
        ];

        for radius in [4, 12, 24] {
            for direction in directions {
                let expected = brute_force_horizontal_offsets(radius, direction);
                let mut actual = Vec::new();
                rebuild_horizontal_selection_offsets(&mut actual, radius, direction);
                let actual = actual.into_iter().collect::<HashSet<_>>();
                assert_eq!(actual, expected, "radius={radius} direction={direction:?}");
            }
        }
    }

    #[test]
    fn incremental_pending_delta_preserves_unchanged_entries() {
        let mut streaming = ChunkStreamingState::default();
        let render_pool = crate::world::chunk_rendering::ChunkRenderPool::default();
        let removed = IVec3::new(0, 0, 0);
        let retained = IVec3::new(1, 0, 0);
        let added = IVec3::new(2, 0, 0);

        streaming.pending.enqueue(removed);
        streaming.pending.enqueue(retained);

        apply_incremental_pending_delta(
            &mut streaming,
            &render_pool,
            &[added],
            &[removed],
        );

        assert_eq!(
            streaming.pending.values_in_order().collect::<Vec<_>>(),
            vec![retained, added]
        );
    }

    #[test]
    fn retired_chunks_exclude_both_live_selection_generations() {
        let far = IVec3::new(4, 0, 0);
        let nearer = IVec3::new(3, 0, 0);
        let reentered_desired = IVec3::new(2, 0, 0);
        let renewed_retention = IVec3::new(1, 0, 0);
        let previous_retained =
            HashSet::from([far, nearer, reentered_desired, renewed_retention]);
        let desired = HashSet::from([reentered_desired]);
        let retained = HashSet::from([renewed_retention]);
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
    fn retained_delta_preserves_previous_selection_union() {
        let previous_desired = HashSet::from([
            IVec3::new(0, 0, 0),
            IVec3::new(1, 0, 0),
            IVec3::new(2, 0, 0),
        ]);
        let desired = HashSet::from([
            IVec3::new(1, 0, 0),
            IVec3::new(2, 0, 0),
            IVec3::new(3, 0, 0),
        ]);
        let retained = previous_desired
            .difference(&desired)
            .copied()
            .collect::<HashSet<_>>();

        let live = desired.union(&retained).copied().collect::<HashSet<_>>();
        let expected = desired
            .union(&previous_desired)
            .copied()
            .collect::<HashSet<_>>();

        assert_eq!(retained, HashSet::from([IVec3::ZERO]));
        assert_eq!(live, expected);
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
        let left = IVec3::new(-3, 0, 0);
        let right = IVec3::new(3, 0, 0);

        let left_key = (priority, left.y, left.z, left.x);
        let right_key = (priority, right.y, right.z, right.x);

        assert!(left_key < right_key);
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
    fn high_player_still_prioritizes_nearer_horizontal_chunks() {
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

        assert!(air_priority < surface_priority);
    }
}
