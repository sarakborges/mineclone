mod path;
mod selection;
mod water_bodies;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use self::{
    path::{RiverEdgeSpec, add_curved_river_edge},
    selection::{
        RiverSelection, build_flow_cache, connected_lake_cells,
        drainage_reaches_water_destination, selected_river_sources,
    },
    water_bodies::{mountain_spring_body, plunge_pool_for_waterfall},
};
use super::{
    constants::RIVER_EDGE_MARGIN_CELLS,
    drainage::{DrainageNetwork, DrainageNode},
    math::{cell_hash, hash_unit, lerp},
    spatial::water_body_intersects_region,
    types::{HydrologySurfaceSample, WaterBody},
};

pub(super) struct RiverSystem {
    pub graph: FeatureGraph,
    pub water_bodies: Vec<WaterBody>,
}

pub(super) fn build_river_system<F>(
    coord: IVec2,
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
    river_weight: f32,
    lake_weight: f32,
    network: &mut DrainageNetwork<'_, F>,
) -> RiverSystem
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut graph = FeatureGraph::default();
    let mut water_bodies = Vec::new();
    let flow_cache = build_flow_cache(coord, network);
    let selection = selected_river_sources(
        &flow_cache,
        seed,
        sea_level,
        water_fluid,
        river_weight,
        lake_weight,
        network,
    );
    let connected_lakes = connected_lake_cells(&selection.lakes, network);
    let ocean_threshold = network.ocean_threshold();
    let mut destination_cache = HashMap::new();

    for dz in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
        for dx in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
            let cell = coord + IVec2::new(dx, dz);
            let source = network.node(cell);

            if source.continentalness <= ocean_threshold {
                continue;
            }

            let reaches_destination = connected_lakes.contains(&cell)
                || drainage_reaches_water_destination(
                    cell,
                    &connected_lakes,
                    network,
                    &mut destination_cache,
                );
            if !reaches_destination {
                continue;
            }

            let spring = selection.springs.contains(&cell).then(|| {
                mountain_spring_body(cell, source, seed, sea_level, water_fluid)
            });
            let lake = selection
                .lakes
                .get(&cell)
                .filter(|_| connected_lakes.contains(&cell))
                .cloned();

            if let Some(body) = spring
                .or(lake)
                .filter(|body| water_body_intersects_region(coord, body))
            {
                water_bodies.push(body);
            }

            if !selection.channels.contains(&cell) {
                continue;
            }

            let Some(downstream_cell) = network.downstream_cell(cell) else {
                continue;
            };
            let downstream = network.node(downstream_cell);
            let flow = flow_cache.get(&cell).copied().unwrap_or(1);
            let downstream_flow = flow_cache.get(&downstream_cell).copied().unwrap_or(flow);
            let source_water_level = selection
                .lakes
                .get(&cell)
                .filter(|_| connected_lakes.contains(&cell))
                .map(|lake| lake.water_level);
            let downstream_water_level = selection
                .lakes
                .get(&downstream_cell)
                .filter(|_| connected_lakes.contains(&downstream_cell))
                .map(|lake| lake.water_level);
            let (downstream, downstream_water_level, downstream_flow) = confluence_target(
                cell,
                downstream_cell,
                downstream,
                downstream_water_level,
                flow,
                downstream_flow,
                seed,
                &selection,
                &flow_cache,
                network,
            );
            let waterfall = add_curved_river_edge(
                &mut graph,
                RiverEdgeSpec {
                    region_coord: coord,
                    source_cell: cell,
                    source,
                    downstream,
                    source_water_level,
                    downstream_water_level,
                    flow,
                    downstream_flow,
                    seed,
                    sea_level,
                },
                |position| network.surface_elevation_at(position),
            );

            if let Some(pool) = waterfall
                .and_then(|waterfall| {
                    plunge_pool_for_waterfall(cell, waterfall, seed, water_fluid)
                })
                .filter(|body| water_body_intersects_region(coord, body))
            {
                water_bodies.push(pool);
            }
        }
    }

    RiverSystem {
        graph,
        water_bodies,
    }
}

#[allow(clippy::too_many_arguments)]
fn confluence_target<F>(
    source_cell: IVec2,
    downstream_cell: IVec2,
    downstream: DrainageNode,
    downstream_water_level: Option<f32>,
    flow: u32,
    downstream_flow: u32,
    seed: u64,
    selection: &RiverSelection,
    flow_cache: &HashMap<IVec2, u32>,
    network: &mut DrainageNetwork<'_, F>,
) -> (DrainageNode, Option<f32>, u32)
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    if downstream_water_level.is_some()
        || downstream_flow <= flow
        || !selection.channels.contains(&downstream_cell)
    {
        return (downstream, downstream_water_level, downstream_flow);
    }

    let Some(next_cell) = network.downstream_cell(downstream_cell) else {
        return (downstream, downstream_water_level, downstream_flow);
    };
    if !selection.channels.contains(&next_cell) {
        return (downstream, downstream_water_level, downstream_flow);
    }

    let next = network.node(next_cell);
    let hash = cell_hash(source_cell, seed ^ 0x3c6e_f372_fe94_f82b);
    let progress = lerp(0.18, 0.42, hash_unit(hash.rotate_left(31)));
    let target = DrainageNode {
        position: downstream.position.lerp(next.position, progress),
        elevation: lerp(downstream.elevation, next.elevation, progress),
        continentalness: lerp(
            downstream.continentalness,
            next.continentalness,
            progress,
        ),
        biome_hydrology: downstream.biome_hydrology,
    };
    let target_flow = flow_cache
        .get(&next_cell)
        .copied()
        .unwrap_or(downstream_flow)
        .max(downstream_flow);

    (target, None, target_flow)
}
