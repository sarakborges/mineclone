use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::super::{
    constants::{
        OCEAN_CONTINENTALNESS_THRESHOLD, RIVER_BASIN_ESCAPE_RADIUS_CELLS,
        RIVER_EDGE_MARGIN_CELLS, RIVER_FLOW_SEARCH_RADIUS, RIVER_FLOW_TRACE_STEPS,
        RIVER_MINIMUM_FLOW,
    },
    drainage::{DrainageNetwork, DrainageNode},
    lake::lake_for_local_basin,
    math::{cell_hash, hash_unit},
    types::{HydrologySurfaceSample, WaterBody},
};

const MOUNTAIN_SPRING_MINIMUM_HEIGHT_ABOVE_SEA: f32 = 30.0;
const MOUNTAIN_SPRING_MINIMUM_LOCAL_RELIEF: f32 = 6.0;
const MOUNTAIN_SPRING_CHANCE: f32 = 0.34;

pub(super) struct RiverSelection {
    pub(super) channels: HashSet<IVec2>,
    pub(super) springs: HashSet<IVec2>,
    pub(super) lakes: HashMap<IVec2, WaterBody>,
}

pub(super) fn drainage_reaches_ocean<F>(
    start: IVec2,
    network: &mut DrainageNetwork<'_, F>,
    cache: &mut HashMap<IVec2, bool>,
) -> bool
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    if let Some(&cached) = cache.get(&start) {
        return cached;
    }

    let mut path = Vec::new();
    let mut current = start;
    let reaches_ocean = loop {
        if let Some(&cached) = cache.get(&current) {
            break cached;
        }

        let node = network.node(current);
        path.push(current);

        if node.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
            break true;
        }
        if path.len() >= RIVER_FLOW_TRACE_STEPS {
            break false;
        }

        let Some(next) = network.downstream_cell(current) else {
            break false;
        };
        current = next;
    };

    for cell in path {
        cache.insert(cell, reaches_ocean);
    }

    reaches_ocean
}

pub(super) fn build_flow_cache<F>(
    coord: IVec2,
    network: &mut DrainageNetwork<'_, F>,
) -> HashMap<IVec2, u32>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let target_radius = RIVER_EDGE_MARGIN_CELLS + RIVER_BASIN_ESCAPE_RADIUS_CELLS;
    let source_radius = target_radius + RIVER_FLOW_SEARCH_RADIUS;
    let mut flow = HashMap::<IVec2, u32>::new();

    for dz in -source_radius..=source_radius {
        for dx in -source_radius..=source_radius {
            let source = coord + IVec2::new(dx, dz);
            let mut current = source;

            for _ in 0..RIVER_FLOW_TRACE_STEPS {
                let relative = current - coord;
                let source_delta = source - current;
                let inside_target =
                    relative.x.abs() <= target_radius && relative.y.abs() <= target_radius;
                let inside_source_radius = source_delta.x.abs() <= RIVER_FLOW_SEARCH_RADIUS
                    && source_delta.y.abs() <= RIVER_FLOW_SEARCH_RADIUS;

                if inside_target && inside_source_radius {
                    *flow.entry(current).or_default() += 1;
                }

                let node = network.node(current);
                if node.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
                    break;
                }

                let Some(next) = network.downstream_cell(current) else {
                    break;
                };
                current = next;
            }
        }
    }

    flow
}

pub(super) fn selected_river_sources<F>(
    flow_cache: &HashMap<IVec2, u32>,
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
    network: &mut DrainageNetwork<'_, F>,
) -> RiverSelection
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut channels = HashSet::new();
    let mut springs = HashSet::new();
    let mut lakes = HashMap::new();

    for (&cell, &flow) in flow_cache {
        let source = network.node(cell);
        if source.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
            continue;
        }

        if source.biome_hydrology.can_generate_lake && source.elevation > sea_level + 1.0 {
            let neighbors = network.neighbor_nodes(cell);
            if let Some(lake) = lake_for_local_basin(
                cell,
                source,
                &neighbors,
                seed,
                sea_level,
                water_fluid,
            ) {
                channels.insert(cell);
                lakes.insert(cell, lake);
            }
        }

        if !source.biome_hydrology.can_generate_river {
            continue;
        }

        if flow >= RIVER_MINIMUM_FLOW {
            channels.insert(cell);
            continue;
        }

        if is_mountain_spring(cell, source, flow, seed, sea_level, network) {
            channels.insert(cell);
            springs.insert(cell);
        }
    }

    extend_selected_downstream(&mut channels, network);
    RiverSelection {
        channels,
        springs,
        lakes,
    }
}

fn is_mountain_spring<F>(
    cell: IVec2,
    source: DrainageNode,
    flow: u32,
    seed: u64,
    sea_level: f32,
    network: &mut DrainageNetwork<'_, F>,
) -> bool
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    if flow >= RIVER_MINIMUM_FLOW
        || source.elevation < sea_level + MOUNTAIN_SPRING_MINIMUM_HEIGHT_ABOVE_SEA
    {
        return false;
    }

    let lowest_neighbor = network
        .neighbor_nodes(cell)
        .into_iter()
        .map(|neighbor| neighbor.elevation)
        .min_by(f32::total_cmp)
        .unwrap_or(source.elevation);
    if source.elevation - lowest_neighbor < MOUNTAIN_SPRING_MINIMUM_LOCAL_RELIEF {
        return false;
    }

    let hash = cell_hash(cell, seed ^ 0x510e_527f_ade6_82d1);
    hash_unit(hash.rotate_left(19)) < MOUNTAIN_SPRING_CHANCE
}

fn extend_selected_downstream<F>(
    selected: &mut HashSet<IVec2>,
    network: &mut DrainageNetwork<'_, F>,
) where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let starts = selected.iter().copied().collect::<Vec<_>>();

    for start in starts {
        let mut current = start;

        for _ in 0..RIVER_FLOW_TRACE_STEPS {
            selected.insert(current);

            let Some(next) = network.downstream_cell(current) else {
                break;
            };
            let downstream = network.node(next);

            if downstream.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
                break;
            }

            current = next;
        }
    }
}
