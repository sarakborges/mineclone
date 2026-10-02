use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
    },
    world::deterministic::{hash_signed, hash_unit, mix_hash_u64},
};

use super::{ChunkGenerationContext, generation_surface_height};

const RIVER_BASIN_CHANCE: f32 = 0.24;
const RIVER_RADIUS: i32 = 2;
const LAKE_RADIUS: i32 = 9;
const RIVER_MEANDER_FRACTION: f32 = 0.18;
const SITE_QUERY_MARGIN: i32 = 2;
const RIVER_HASH_SALT: u64 = 0x6a09_e667_f3bc_c909;
const EDGE_HASH_SALT: u64 = 0xbb67_ae85_84ca_a73b;

#[derive(Clone, Copy, Debug)]
struct DrainageSite {
    position: IVec2,
    surface_height: i32,
    ocean: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum DrainageSink {
    Ocean(IVec2),
    Lake(IVec2),
}

impl DrainageSink {
    fn cell(self) -> IVec2 {
        match self {
            Self::Ocean(cell) | Self::Lake(cell) => cell,
        }
    }
}

pub(super) fn rasterize_river_network(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
) {
    let Some(ocean_biome) = context.dimension.ocean_biome.as_deref() else {
        return;
    };
    let Some(water_id) = context.fluids.id_of(&context.dimension.sea_fluid) else {
        return;
    };

    let spacing = context.biome_field.surface_site_spacing();
    if spacing.x <= 0.0 || spacing.y <= 0.0 {
        return;
    }

    let chunk_min = chunk_origin.xz();
    let chunk_max = chunk_min + IVec2::splat(CHUNK_SIZE as i32 - 1);
    let minimum_cell = IVec2::new(
        (chunk_min.x as f32 / spacing.x).floor() as i32 - SITE_QUERY_MARGIN,
        (chunk_min.y as f32 / spacing.y).floor() as i32 - SITE_QUERY_MARGIN,
    );
    let maximum_cell = IVec2::new(
        (chunk_max.x as f32 / spacing.x).ceil() as i32 + SITE_QUERY_MARGIN,
        (chunk_max.y as f32 / spacing.y).ceil() as i32 + SITE_QUERY_MARGIN,
    );

    let mut site_cache = HashMap::<IVec2, DrainageSite>::new();
    let mut sink_cache = HashMap::<IVec2, DrainageSink>::new();
    let mut water_positions = HashSet::<IVec3>::new();
    let mut clear_positions = HashSet::<IVec3>::new();

    for cell_z in minimum_cell.y..=maximum_cell.y {
        for cell_x in minimum_cell.x..=maximum_cell.x {
            let cell = IVec2::new(cell_x, cell_z);
            let site = drainage_site(cell, ocean_biome, context, &mut site_cache);
            if site.ocean {
                continue;
            }

            let sink = drainage_sink(
                cell,
                ocean_biome,
                context,
                &mut site_cache,
                &mut sink_cache,
            );
            if !river_basin_is_active(context.biome_field.seed(), sink) {
                continue;
            }

            match downstream_site(cell, ocean_biome, context, &mut site_cache) {
                Some(next_cell) => {
                    let next = drainage_site(next_cell, ocean_biome, context, &mut site_cache);
                    rasterize_edge_points(
                        cell,
                        site.position,
                        next.position,
                        ocean_biome,
                        chunk_min,
                        chunk_max,
                        context,
                        &mut water_positions,
                        &mut clear_positions,
                    );
                }
                None if sink == DrainageSink::Lake(cell) => {
                    rasterize_lake_points(
                        site.position,
                        chunk_min,
                        chunk_max,
                        context,
                        &mut water_positions,
                        &mut clear_positions,
                    );
                }
                None => {}
            }
        }
    }

    if water_positions.is_empty() && clear_positions.is_empty() {
        return;
    }

    chunk.edit_structure_content(|chunk| {
        for position in clear_positions {
            let local = position - chunk_origin;
            if voxel_is_inside_chunk(local) {
                chunk.clear_block(local.x as usize, local.y as usize, local.z as usize);
                chunk.clear_fluid(local.x as usize, local.y as usize, local.z as usize);
            }
        }
        let source = FluidCell::source(water_id, MAX_FLUID_LEVEL);
        for position in water_positions {
            let local = position - chunk_origin;
            if voxel_is_inside_chunk(local) {
                chunk.clear_block(local.x as usize, local.y as usize, local.z as usize);
                chunk.set_fluid(
                    local.x as usize,
                    local.y as usize,
                    local.z as usize,
                    source,
                );
            }
        }
    });
}

fn drainage_site(
    cell: IVec2,
    ocean_biome: &str,
    context: &ChunkGenerationContext<'_>,
    cache: &mut HashMap<IVec2, DrainageSite>,
) -> DrainageSite {
    if let Some(site) = cache.get(&cell).copied() {
        return site;
    }

    let position = context.biome_field.surface_site_position(cell).round().as_ivec2();
    let sample = context
        .biome_field
        .sample_surface(position.as_vec2() + Vec2::splat(0.5));
    let site = DrainageSite {
        position,
        surface_height: generation_surface_height(position, context),
        ocean: sample.primary_id == ocean_biome,
    };
    cache.insert(cell, site);
    site
}

fn downstream_site(
    cell: IVec2,
    ocean_biome: &str,
    context: &ChunkGenerationContext<'_>,
    cache: &mut HashMap<IVec2, DrainageSite>,
) -> Option<IVec2> {
    let current = drainage_site(cell, ocean_biome, context, cache);
    if current.ocean {
        return None;
    }

    let mut best_ocean = None::<(u64, IVec2)>;
    let mut best_land = None::<(i32, u64, IVec2)>;
    for dz in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dz == 0 {
                continue;
            }
            let candidate_cell = cell + IVec2::new(dx, dz);
            let candidate = drainage_site(candidate_cell, ocean_biome, context, cache);
            let tie = cell_hash(context.biome_field.seed(), candidate_cell, EDGE_HASH_SALT);

            if candidate.ocean {
                if best_ocean.is_none_or(|(current_tie, _)| tie < current_tie) {
                    best_ocean = Some((tie, candidate_cell));
                }
                continue;
            }
            if candidate.surface_height >= current.surface_height {
                continue;
            }

            if best_land.is_none_or(|(best_height, best_tie, _)| {
                (candidate.surface_height, tie) < (best_height, best_tie)
            }) {
                best_land = Some((candidate.surface_height, tie, candidate_cell));
            }
        }
    }

    best_ocean
        .map(|(_, cell)| cell)
        .or_else(|| best_land.map(|(_, _, cell)| cell))
}

fn drainage_sink(
    start: IVec2,
    ocean_biome: &str,
    context: &ChunkGenerationContext<'_>,
    site_cache: &mut HashMap<IVec2, DrainageSite>,
    sink_cache: &mut HashMap<IVec2, DrainageSink>,
) -> DrainageSink {
    if let Some(sink) = sink_cache.get(&start).copied() {
        return sink;
    }

    let mut path = Vec::new();
    let mut seen = HashSet::new();
    let mut current = start;
    let sink = loop {
        if let Some(sink) = sink_cache.get(&current).copied() {
            break sink;
        }
        if !seen.insert(current) {
            break DrainageSink::Lake(current);
        }

        path.push(current);
        let site = drainage_site(current, ocean_biome, context, site_cache);
        if site.ocean {
            break DrainageSink::Ocean(current);
        }

        match downstream_site(current, ocean_biome, context, site_cache) {
            Some(next) => current = next,
            None => break DrainageSink::Lake(current),
        }
    };

    for cell in path {
        sink_cache.insert(cell, sink);
    }
    sink
}

fn river_basin_is_active(seed: u64, sink: DrainageSink) -> bool {
    hash_unit(cell_hash(seed, sink.cell(), RIVER_HASH_SALT)) < RIVER_BASIN_CHANCE
}

#[allow(clippy::too_many_arguments)]
fn rasterize_edge_points(
    cell: IVec2,
    start: IVec2,
    end: IVec2,
    ocean_biome: &str,
    chunk_min: IVec2,
    chunk_max: IVec2,
    context: &ChunkGenerationContext<'_>,
    water: &mut HashSet<IVec3>,
    clear: &mut HashSet<IVec3>,
) {
    let hash = cell_hash(context.biome_field.seed(), cell, EDGE_HASH_SALT);
    let start_vec = start.as_vec2();
    let end_vec = end.as_vec2();
    let delta = end_vec - start_vec;
    let distance = delta.length();
    if distance <= f32::EPSILON {
        return;
    }

    let perpendicular = Vec2::new(-delta.y, delta.x).normalize_or_zero();
    let control = (start_vec + end_vec) * 0.5
        + perpendicular * distance * RIVER_MEANDER_FRACTION * hash_signed(hash);
    let steps = distance.ceil().max(1.0) as usize;
    let expanded_min = chunk_min - IVec2::splat(RIVER_RADIUS + 1);
    let expanded_max = chunk_max + IVec2::splat(RIVER_RADIUS + 1);
    let mut previous = None;

    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let omt = 1.0 - t;
        let point = start_vec * (omt * omt) + control * (2.0 * omt * t) + end_vec * (t * t);
        let center = point.round().as_ivec2();
        if previous == Some(center) {
            continue;
        }
        previous = Some(center);

        let sample = context
            .biome_field
            .sample_surface(center.as_vec2() + Vec2::splat(0.5));
        if sample.primary_id == ocean_biome {
            break;
        }
        if center.x < expanded_min.x
            || center.x > expanded_max.x
            || center.y < expanded_min.y
            || center.y > expanded_max.y
        {
            continue;
        }

        rasterize_water_brush(center, RIVER_RADIUS, None, chunk_min, chunk_max, context, water, clear);
    }
}

fn rasterize_lake_points(
    center: IVec2,
    chunk_min: IVec2,
    chunk_max: IVec2,
    context: &ChunkGenerationContext<'_>,
    water: &mut HashSet<IVec3>,
    clear: &mut HashSet<IVec3>,
) {
    let water_y = generation_surface_height(center, context) - 1;
    rasterize_water_brush(
        center,
        LAKE_RADIUS,
        Some(water_y),
        chunk_min,
        chunk_max,
        context,
        water,
        clear,
    );
}

#[allow(clippy::too_many_arguments)]
fn rasterize_water_brush(
    center: IVec2,
    radius: i32,
    fixed_water_y: Option<i32>,
    chunk_min: IVec2,
    chunk_max: IVec2,
    context: &ChunkGenerationContext<'_>,
    water: &mut HashSet<IVec3>,
    clear: &mut HashSet<IVec3>,
) {
    let radius_squared = i64::from(radius) * i64::from(radius);
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if i64::from(dx) * i64::from(dx) + i64::from(dz) * i64::from(dz) > radius_squared {
                continue;
            }
            let horizontal = center + IVec2::new(dx, dz);
            if horizontal.x < chunk_min.x
                || horizontal.x > chunk_max.x
                || horizontal.y < chunk_min.y
                || horizontal.y > chunk_max.y
            {
                continue;
            }

            let surface_ground = generation_surface_height(horizontal, context) - 1;
            let water_y = fixed_water_y.unwrap_or(surface_ground);
            if let Some(fixed) = fixed_water_y
                && surface_ground >= fixed
            {
                for y in fixed..=surface_ground {
                    clear.insert(IVec3::new(horizontal.x, y, horizontal.y));
                }
            }
            water.insert(IVec3::new(horizontal.x, water_y, horizontal.y));
        }
    }
}

fn voxel_is_inside_chunk(local: IVec3) -> bool {
    let size = CHUNK_SIZE as i32;
    (0..size).contains(&local.x)
        && (0..size).contains(&local.y)
        && (0..size).contains(&local.z)
}

fn cell_hash(seed: u64, cell: IVec2, salt: u64) -> u64 {
    let mut hash = seed ^ salt;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    mix_hash_u64(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basin_selection_is_stable_for_same_sink() {
        let sink = DrainageSink::Lake(IVec2::new(12, -7));
        assert_eq!(
            river_basin_is_active(42, sink),
            river_basin_is_active(42, sink)
        );
    }

    #[test]
    fn sink_cell_is_shared_by_ocean_and_lake_variants() {
        let cell = IVec2::new(-3, 9);
        assert_eq!(DrainageSink::Ocean(cell).cell(), cell);
        assert_eq!(DrainageSink::Lake(cell).cell(), cell);
    }
}
