use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::super::{
    constants::{
        RIVER_BASIN_ESCAPE_RADIUS_CELLS, RIVER_EDGE_MARGIN_CELLS, RIVER_FLOW_SEARCH_RADIUS,
        RIVER_FLOW_TRACE_STEPS, RIVER_MINIMUM_FLOW,
    },
    drainage::{DrainageNetwork, DrainageNode},
    lake::{LakeBasinContext, lake_for_local_basin},
    math::{cell_hash, hash_unit},
    types::{HydrologySurfaceSample, WaterBody},
};

const MOUNTAIN_SPRING_MINIMUM_HEIGHT_ABOVE_SEA: f32 = 36.0;
const MOUNTAIN_SPRING_MINIMUM_LOCAL_RELIEF: f32 = 10.0;
const MOUNTAIN_SPRING_CHANCE: f32 = 0.42;

pub(super) struct RiverSelection {
    pub(super) channels: HashSet<IVec2>,
    pub(super) springs: HashSet<IVec2>,
    pub(super) lakes: HashMap<IVec2, WaterBody>,
}

pub(super) fn connected_lake_cells<F>(
    lakes: &HashMap<IVec2, WaterBody>,
    network: &mut DrainageNetwork<'_, F>,
) -> HashSet<IVec2>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut connected = HashSet::new();

    for &start in lakes.keys() {
        let mut current = start;

        for _ in 0..RIVER_FLOW_TRACE_STEPS {
            let Some(next) = network.downstream_cell(current) else {
                break;
            };
            current = next;
            let node = network.node(current);

            if network.is_wet_ocean(node) {
                connected.insert(start);
                break;
            }

            if current != start && lakes.contains_key(&current) {
                connected.insert(start);
                connected.insert(current);
                break;
            }
        }
    }

    connected
}

pub(super) fn drainage_reaches_water_destination<F>(
    start: IVec2,
    lake_cells: &HashSet<IVec2>,
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
    let mut trace_limit_exhausted = false;
    let reaches_destination = loop {
        if current != start && lake_cells.contains(&current) {
            break true;
        }
        if let Some(&cached) = cache.get(&current) {
            break cached;
        }

        let node = network.node(current);
        path.push(current);

        if network.is_wet_ocean(node) {
            break true;
        }
        if path.len() >= RIVER_FLOW_TRACE_STEPS {
            trace_limit_exhausted = true;
            break false;
        }

        let Some(next) = network.downstream_cell(current) else {
            break false;
        };
        current = next;
    };

    cache_reachability(
        &path,
        reaches_destination,
        trace_limit_exhausted,
        cache,
    );
    reaches_destination
}

fn cache_reachability(
    path: &[IVec2],
    reaches_destination: bool,
    trace_limit_exhausted: bool,
    cache: &mut HashMap<IVec2, bool>,
) {
    if trace_limit_exhausted {
        // An exhausted trace is inconclusive, including for its starting cell.
        // Caching false there would override a later proven destination from
        // a downstream trace within the same region build.
        return;
    }

    for &cell in path {
        cache.insert(cell, reaches_destination);
    }
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
                if network.is_wet_ocean(node) {
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
    river_weight: f32,
    lake_weight: f32,
    network: &mut DrainageNetwork<'_, F>,
) -> RiverSelection
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut channels = HashSet::new();
    let mut springs = HashSet::new();
    let mut lakes = HashMap::new();
    let ocean_threshold = network.ocean_threshold();
    let river_weight = river_weight.clamp(0.0, 1.0);
    let lake_weight = lake_weight.clamp(0.0, 1.0);
    let river_flow_threshold = river_flow_threshold(river_weight);
    let lake_context = LakeBasinContext {
        seed,
        sea_level,
        water_fluid,
        ocean_threshold,
        lake_weight,
    };

    for (&cell, &flow) in flow_cache {
        let source = network.node(cell);
        if network.is_wet_ocean(source) {
            continue;
        }

        if source.biome_hydrology.can_generate_lake && source.elevation > sea_level + 1.0 {
            let neighbors = network.neighbor_nodes(cell);
            if let Some(lake) = lake_for_local_basin(cell, source, &neighbors, &lake_context) {
                channels.insert(cell);
                lakes.insert(cell, lake);
            }
        }

        if !source.biome_hydrology.can_generate_river {
            continue;
        }

        if flow >= river_flow_threshold {
            channels.insert(cell);
            continue;
        }

        if is_mountain_spring(cell, source, flow, seed, sea_level, river_weight, network) {
            channels.insert(cell);
            springs.insert(cell);
        }
    }

    keep_only_complete_downstream_paths(&mut channels, &lakes, network);
    springs.retain(|cell| channels.contains(cell));

    RiverSelection {
        channels,
        springs,
        lakes,
    }
}

fn river_flow_threshold(river_weight: f32) -> u32 {
    if river_weight <= f32::EPSILON {
        return u32::MAX;
    }

    ((RIVER_MINIMUM_FLOW as f32 / river_weight.max(0.15)).ceil() as u32).max(RIVER_MINIMUM_FLOW)
}

fn is_mountain_spring<F>(
    cell: IVec2,
    source: DrainageNode,
    flow: u32,
    seed: u64,
    sea_level: f32,
    river_weight: f32,
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
    hash_unit(hash.rotate_left(19)) < MOUNTAIN_SPRING_CHANCE * river_weight
}

fn keep_only_complete_downstream_paths<F>(
    selected: &mut HashSet<IVec2>,
    lakes: &HashMap<IVec2, WaterBody>,
    network: &mut DrainageNetwork<'_, F>,
) where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let starts = selected.iter().copied().collect::<Vec<_>>();
    let mut complete = HashSet::new();

    for start in starts {
        let mut current = start;
        let mut path = Vec::new();
        let mut reaches_destination = false;

        for _ in 0..RIVER_FLOW_TRACE_STEPS {
            let node = network.node(current);

            if network.is_wet_ocean(node) {
                reaches_destination = true;
                break;
            }
            if current != start && lakes.contains_key(&current) {
                reaches_destination = true;
                break;
            }
            if !node.biome_hydrology.can_generate_river {
                break;
            }

            path.push(current);
            let Some(next) = network.downstream_cell(current) else {
                break;
            };
            current = next;
        }

        if reaches_destination {
            complete.extend(path);
        }
    }

    *selected = complete;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_trace_does_not_cache_an_unproven_dead_end() {
        let path = (0..RIVER_FLOW_TRACE_STEPS)
            .map(|index| IVec2::new(index as i32, 0))
            .collect::<Vec<_>>();
        let mut cache = HashMap::new();

        cache_reachability(&path, false, true, &mut cache);

        assert!(cache.is_empty());

        // A subsequent shorter trace may prove that the outlet is reachable.
        // No stale negative entry may mask that later result.
        cache_reachability(&path[1..], true, false, &mut cache);
        assert_eq!(cache.get(&path[0]), None);
        assert!(path[1..].iter().all(|cell| cache.get(cell) == Some(&true)));
    }

    #[test]
    fn proven_destination_or_dead_end_caches_complete_path() {
        let start = IVec2::ZERO;
        let path = [start, IVec2::X, IVec2::new(2, 0)];
        let mut cache = HashMap::new();

        cache_reachability(&path, true, false, &mut cache);
        assert!(path.iter().all(|cell| cache.get(cell) == Some(&true)));

        cache.clear();
        cache_reachability(&path, false, false, &mut cache);
        assert!(path.iter().all(|cell| cache.get(cell) == Some(&false)));
    }
}
