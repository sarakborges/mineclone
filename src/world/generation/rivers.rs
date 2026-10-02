use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    content::structure::StructureDefinition,
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
    },
    world::{
        deterministic::{hash_signed, hash_unit, mix_hash_u64},
        new_world::WorldGenerationMode,
    },
};

use super::{ChunkGenerationContext, generation_surface_height};

const RIVER_HASH_SALT: u64 = 0x6a09_e667_f3bc_c909;
const EDGE_HASH_SALT: u64 = 0xbb67_ae85_84ca_a73b;
const STAMP_HASH_SALT: u64 = 0x3c6e_f372_fe94_f82b;

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RiverStampKind {
    Channel,
    Lake,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct RiverStamp {
    kind: RiverStampKind,
    center: IVec2,
    origin_y: i32,
}

pub(super) fn rasterize_river_network(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
) {
    if context.world_generation.mode() != WorldGenerationMode::Normal {
        return;
    }
    let Some(config) = context.dimension.river_network.as_ref() else {
        return;
    };
    let Some(ocean_biome) = context.dimension.ocean_biome.as_deref() else {
        return;
    };

    let spacing = context.biome_field.surface_site_spacing();
    if spacing.x <= 0.0 || spacing.y <= 0.0 {
        return;
    }

    let channel_extent =
        structure_reference_horizontal_extent(context, &config.channel_structure);
    let lake_extent = structure_reference_horizontal_extent(context, &config.lake_structure);
    let maximum_piece_extent = channel_extent.max(lake_extent) as f32;
    let maximum_adjacent_edge = spacing.length();
    let minimum_site_spacing = spacing.x.min(spacing.y);
    let site_query_margin = 1
        + ((maximum_piece_extent + maximum_adjacent_edge * config.meander)
            / minimum_site_spacing)
            .ceil() as i32;

    let chunk_min = chunk_origin.xz();
    let chunk_max = chunk_min + IVec2::splat(CHUNK_SIZE as i32 - 1);
    let minimum_cell = IVec2::new(
        (chunk_min.x as f32 / spacing.x).floor() as i32 - site_query_margin,
        (chunk_min.y as f32 / spacing.y).floor() as i32 - site_query_margin,
    );
    let maximum_cell = IVec2::new(
        (chunk_max.x as f32 / spacing.x).ceil() as i32 + site_query_margin,
        (chunk_max.y as f32 / spacing.y).ceil() as i32 + site_query_margin,
    );

    let mut site_cache = HashMap::<IVec2, DrainageSite>::new();
    let mut sink_cache = HashMap::<IVec2, DrainageSink>::new();
    let mut stamps = HashSet::<RiverStamp>::new();

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
            if !river_basin_is_active(context.biome_field.seed(), sink, config.basin_chance) {
                continue;
            }

            match downstream_site(cell, ocean_biome, context, &mut site_cache) {
                Some(next_cell) => {
                    let next = drainage_site(next_cell, ocean_biome, context, &mut site_cache);
                    collect_edge_stamps(
                        cell,
                        site.position,
                        next.position,
                        ocean_biome,
                        chunk_min,
                        chunk_max,
                        channel_extent,
                        config.meander,
                        context,
                        &mut stamps,
                    );
                }
                None if sink == DrainageSink::Lake(cell) => {
                    let origin_y = generation_surface_height(site.position, context) - 1;
                    if centered_piece_may_intersect_chunk(
                        site.position,
                        lake_extent,
                        chunk_min,
                        chunk_max,
                    ) {
                        stamps.insert(RiverStamp {
                            kind: RiverStampKind::Lake,
                            center: site.position,
                            origin_y,
                        });
                    }
                }
                None => {}
            }
        }
    }

    let mut stamps = stamps.into_iter().collect::<Vec<_>>();
    stamps.sort_unstable_by_key(|stamp| {
        (
            match stamp.kind {
                RiverStampKind::Channel => 0_u8,
                RiverStampKind::Lake => 1_u8,
            },
            stamp.center.y,
            stamp.center.x,
            stamp.origin_y,
        )
    });

    for stamp in stamps {
        let reference = match stamp.kind {
            RiverStampKind::Channel => config.channel_structure.as_str(),
            RiverStampKind::Lake => config.lake_structure.as_str(),
        };
        let hash = cell_hash(context.biome_field.seed(), stamp.center, STAMP_HASH_SALT);
        let structure = context
            .structures
            .select_for_reference(reference, hash)
            .unwrap_or_else(|| panic!("river network references missing structure: {reference}"));
        let rotation = structure.rotation_for_hash(hash.rotate_left(23));
        stamp_authored_river_piece(
            chunk,
            chunk_origin,
            context,
            structure,
            rotation,
            IVec3::new(stamp.center.x, stamp.origin_y, stamp.center.y),
        );
    }
}

fn structure_reference_horizontal_extent(
    context: &ChunkGenerationContext<'_>,
    reference: &str,
) -> i32 {
    context
        .structures
        .reference_members(reference)
        .unwrap_or_else(|| panic!("river network references missing structure: {reference}"))
        .into_iter()
        .flat_map(|structure| {
            structure.supported_rotations().iter().map(move |&rotation| {
                let (minimum, maximum) = structure.horizontal_bounds_for_rotation(rotation);
                minimum
                    .x
                    .unsigned_abs()
                    .max(minimum.y.unsigned_abs())
                    .max(maximum.x.unsigned_abs())
                    .max(maximum.y.unsigned_abs()) as i32
            })
        })
        .max()
        .unwrap_or(0)
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

fn river_basin_is_active(seed: u64, sink: DrainageSink, chance: f32) -> bool {
    hash_unit(cell_hash(seed, sink.cell(), RIVER_HASH_SALT)) < chance
}

#[allow(clippy::too_many_arguments)]
fn collect_edge_stamps(
    cell: IVec2,
    start: IVec2,
    end: IVec2,
    ocean_biome: &str,
    chunk_min: IVec2,
    chunk_max: IVec2,
    channel_extent: i32,
    meander: f32,
    context: &ChunkGenerationContext<'_>,
    stamps: &mut HashSet<RiverStamp>,
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
    let control =
        (start_vec + end_vec) * 0.5 + perpendicular * distance * meander * hash_signed(hash);
    let steps = distance.ceil().max(1.0) as usize;
    let expanded_min = chunk_min - IVec2::splat(channel_extent);
    let expanded_max = chunk_max + IVec2::splat(channel_extent);
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

        stamps.insert(RiverStamp {
            kind: RiverStampKind::Channel,
            center,
            origin_y: generation_surface_height(center, context) - 1,
        });
    }
}

fn centered_piece_may_intersect_chunk(
    center: IVec2,
    extent: i32,
    chunk_min: IVec2,
    chunk_max: IVec2,
) -> bool {
    center.x + extent >= chunk_min.x
        && center.x - extent <= chunk_max.x
        && center.y + extent >= chunk_min.y
        && center.y - extent <= chunk_max.y
}

fn stamp_authored_river_piece(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
    structure: &StructureDefinition,
    rotation: crate::content::structure::StructureRotation,
    origin: IVec3,
) {
    chunk.edit_structure_content(|chunk| {
        for voxel in structure.voxels() {
            let world_position = origin + rotation.rotate_offset(voxel.offset);
            let local = world_position - chunk_origin;
            if !voxel_is_inside_chunk(local) {
                continue;
            }

            assert!(
                voxel.block_id.is_none()
                    && structure.objects_for_voxel(voxel).is_empty()
                    && !structure.layers_only_voxel(voxel),
                "river network structure {} may only contain fluid or clear voxels",
                structure.id
            );

            let x = local.x as usize;
            let y = local.y as usize;
            let z = local.z as usize;
            if let Some(fluid_reference) = structure.fluid_for_voxel(voxel) {
                let fluid_id = context.fluids.id_of(fluid_reference).unwrap_or_else(|| {
                    panic!(
                        "river network structure {} references missing fluid: {}",
                        structure.id, fluid_reference
                    )
                });
                chunk.clear_block(x, y, z);
                chunk.set_fluid(x, y, z, FluidCell::source(fluid_id, MAX_FLUID_LEVEL));
            } else if structure.clears_voxel(voxel) {
                chunk.clear_block(x, y, z);
                chunk.clear_fluid(x, y, z);
            } else {
                panic!(
                    "river network structure {} contains unsupported empty voxel payload",
                    structure.id
                );
            }
        }

        for world_position in structure.clear_above_positions(rotation, origin) {
            let local = world_position - chunk_origin;
            if !voxel_is_inside_chunk(local) {
                continue;
            }
            chunk.clear_block(local.x as usize, local.y as usize, local.z as usize);
            chunk.clear_fluid(local.x as usize, local.y as usize, local.z as usize);
        }
    });
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
            river_basin_is_active(42, sink, 0.24),
            river_basin_is_active(42, sink, 0.24)
        );
    }

    #[test]
    fn basin_chance_controls_activation() {
        let sink = DrainageSink::Lake(IVec2::new(12, -7));
        assert!(!river_basin_is_active(42, sink, 0.0));
        assert!(river_basin_is_active(42, sink, 1.0));
    }

    #[test]
    fn sink_cell_is_shared_by_ocean_and_lake_variants() {
        let cell = IVec2::new(-3, 9);
        assert_eq!(DrainageSink::Ocean(cell).cell(), cell);
        assert_eq!(DrainageSink::Lake(cell).cell(), cell);
    }
}
