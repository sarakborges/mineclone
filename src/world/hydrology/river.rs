mod path;
mod selection;
mod water_bodies;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use self::{
    path::{RiverEdgeSpec, add_curved_river_edge},
    selection::{
        RiverSelection, build_flow_cache, connected_lake_cells, drainage_reaches_water_destination,
        selected_river_sources,
    },
    water_bodies::{confluence_lake, mountain_spring_body, plunge_pool_for_waterfall},
};
use super::{
    constants::RIVER_EDGE_MARGIN_CELLS,
    drainage::{DrainageNetwork, DrainageNode},
    spatial::water_body_intersects_region,
    types::{HydrologySurfaceSample, WaterBody},
};

const CONFLUENCE_LAKE_MINIMUM_INCOMING_RIVERS: usize = 3;

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
    let confluences = direct_confluence_counts(&selection, network);
    let connected_lakes = connected_lake_cells(&selection.lakes, network);
    let ocean_threshold = network.ocean_threshold();
    let mut destination_cache = HashMap::new();
    let mut confluence_water_levels = HashMap::new();

    for (&cell, &incoming_rivers) in &confluences {
        if incoming_rivers < CONFLUENCE_LAKE_MINIMUM_INCOMING_RIVERS
            || selection.lakes.contains_key(&cell)
        {
            continue;
        }

        let source = network.node(cell);
        if source.continentalness <= ocean_threshold {
            continue;
        }

        let flow = flow_cache
            .get(&cell)
            .copied()
            .unwrap_or(incoming_rivers as u32);
        let body = confluence_lake(
            cell,
            source,
            incoming_rivers,
            flow,
            seed,
            sea_level,
            water_fluid,
        );
        confluence_water_levels.insert(cell, body.water_level);

        if water_body_intersects_region(coord, &body) {
            water_bodies.push(body);
        }
    }

    for dz in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
        for dx in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
            let cell = coord + IVec2::new(dx, dz);
            let source = network.node(cell);

            if network.is_wet_ocean(source) {
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

            let spring = selection
                .springs
                .contains(&cell)
                .then(|| mountain_spring_body(cell, source, seed, sea_level, water_fluid));
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
                .map(|lake| lake.water_level)
                .or_else(|| confluence_water_levels.get(&cell).copied());
            let downstream_water_level = selection
                .lakes
                .get(&downstream_cell)
                .filter(|_| connected_lakes.contains(&downstream_cell))
                .map(|lake| lake.water_level)
                .or_else(|| confluence_water_levels.get(&downstream_cell).copied());

            // Every incoming edge must end at the same authoritative drainage
            // node that the trunk's outgoing edge starts from. The previous
            // tributary offset interpolated along a straight chord while the
            // trunk meandered away from that chord, leaving detached mouths.
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
                .and_then(|waterfall| plunge_pool_for_waterfall(cell, waterfall, seed, water_fluid))
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

fn direct_confluence_counts<F>(
    selection: &RiverSelection,
    network: &mut DrainageNetwork<'_, F>,
) -> HashMap<IVec2, usize>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut incoming = HashMap::new();

    for &source in &selection.channels {
        let Some(target) = network.downstream_cell(source) else {
            continue;
        };
        if selection.channels.contains(&target) {
            *incoming.entry(target).or_insert(0) += 1;
        }
    }

    incoming
}
