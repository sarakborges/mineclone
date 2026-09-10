use bevy::prelude::*;

use super::{
    constants::{
        LAKE_CARVE_DEPTH, LAKE_CHANCE, LAKE_MAXIMUM_RADIUS, LAKE_MINIMUM_RADIUS,
        LAKE_MINIMUM_RELIEF, OCEAN_CONTINENTALNESS_THRESHOLD,
    },
    drainage::DrainageNode,
    math::{cell_hash, hash_unit, lerp},
    types::WaterBody,
};

pub(super) fn lake_for_local_basin(
    cell: IVec2,
    source: DrainageNode,
    neighbors: &[DrainageNode],
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
) -> Option<WaterBody> {
    lake_for_basin(
        cell,
        source,
        neighbors,
        seed,
        sea_level,
        water_fluid,
        false,
    )
}

pub(super) fn terminal_lake_for_local_basin(
    cell: IVec2,
    source: DrainageNode,
    neighbors: &[DrainageNode],
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
) -> Option<WaterBody> {
    lake_for_basin(
        cell,
        source,
        neighbors,
        seed,
        sea_level,
        water_fluid,
        true,
    )
}

fn lake_for_basin(
    cell: IVec2,
    source: DrainageNode,
    neighbors: &[DrainageNode],
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
    terminal: bool,
) -> Option<WaterBody> {
    if !source.biome_hydrology.can_generate_lake
        || source.elevation <= sea_level + 1.0
        || source.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD
    {
        return None;
    }

    let spill = neighbors
        .iter()
        .map(|neighbor| neighbor.elevation)
        .min_by(f32::total_cmp)?;
    let relief = (spill - source.elevation).max(0.0);

    if !terminal && relief < LAKE_MINIMUM_RELIEF {
        return None;
    }

    let hash = cell_hash(cell, seed ^ 0xbb67_ae85_84ca_a73b);
    let lake_chance = (LAKE_CHANCE * source.biome_hydrology.lake_chance_multiplier).clamp(0.0, 1.0);

    if !terminal && hash_unit(hash.rotate_left(17)) > lake_chance {
        return None;
    }

    let radius_x = lerp(
        LAKE_MINIMUM_RADIUS,
        LAKE_MAXIMUM_RADIUS,
        hash_unit(hash.rotate_left(29)),
    );
    let radius_z = lerp(
        LAKE_MINIMUM_RADIUS,
        LAKE_MAXIMUM_RADIUS,
        hash_unit(hash.rotate_left(43)),
    );
    let water_level = source.elevation + relief.min(5.0) * 0.7;

    Some(WaterBody {
        center: source.position,
        radius: Vec2::new(radius_x, radius_z),
        water_level,
        carve_depth: LAKE_CARVE_DEPTH,
        fluid_id: water_fluid.to_owned(),
    })
}
