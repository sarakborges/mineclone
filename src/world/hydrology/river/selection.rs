use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::super::{
    constants::{
        RIVER_BASIN_ESCAPE_RADIUS_CELLS, RIVER_EDGE_MARGIN_CELLS, RIVER_FLOW_SEARCH_RADIUS,
        RIVER_FLOW_TRACE_STEPS,
    },
    drainage::DrainageNetwork,
    lake::{LakeBasinContext, lake_for_local_basin},
    types::{HydrologySurfaceSample, WaterBody},
};


pub(super) struct RiverSelection {
    pub(super) channels: HashSet<IVec2>,
    pub(super) lakes: HashMap<IVec2, WaterBody>,
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
    let mut lakes = HashMap::new();
    let lake_context = LakeBasinContext {
        seed,
        sea_level,
        water_fluid,
        lake_weight: lake_weight.clamp(0.0, 1.0),
    };

    for &cell in flow_cache.keys() {
        let source = network.node(cell);
        if network.is_wet_ocean(source)
            || !source.biome_hydrology.can_generate_lake
            || source.elevation <= sea_level + 1.0
        {
            continue;
        }

        let neighbors = network.neighbor_nodes(cell);
        if let Some(lake) = lake_for_local_basin(cell, source, &neighbors, &lake_context) {
            lakes.insert(cell, lake);
        }
    }

    let channels = if river_weight <= f32::EPSILON {
        HashSet::new()
    } else {
        complete_water_body_corridors(&lakes, network)
    };

    RiverSelection { channels, lakes }
}

fn complete_water_body_corridors<F>(
    lakes: &HashMap<IVec2, WaterBody>,
    network: &mut DrainageNetwork<'_, F>,
) -> HashSet<IVec2>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut complete = HashSet::new();

    for &start in lakes.keys() {
        let mut current = start;
        let mut path = Vec::new();

        for _ in 0..RIVER_FLOW_TRACE_STEPS {
            let node = network.node(current);

            if current != start && network.is_wet_ocean(node) {
                complete.extend(path);
                break;
            }
            if current != start && lakes.contains_key(&current) {
                complete.extend(path);
                break;
            }
            if current != start && !node.biome_hydrology.can_generate_river {
                break;
            }

            path.push(current);
            let Some(next) = network.downstream_cell(current) else {
                break;
            };
            current = next;
        }
    }

    complete
}

