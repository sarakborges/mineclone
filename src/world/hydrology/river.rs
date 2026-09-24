mod path;
mod selection;
mod water_bodies;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use self::{
    path::{RiverEdgeSpec, add_curved_river_edge},
    selection::{RiverSelection, build_flow_cache, selected_river_sources},
    water_bodies::{confluence_lake, plunge_pool_for_waterfall},
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

#[allow(clippy::too_many_arguments)]
pub(super) fn build_river_system<F>(
    coord: IVec2,
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
    river_weight: f32,
    lake_weight: f32,
    spawn_lakes: bool,
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
    let ocean_threshold = network.ocean_threshold();
    let mut confluence_bodies = HashMap::new();

    for (&cell, &incoming_rivers) in &confluences {
        if !spawn_lakes
            || incoming_rivers < CONFLUENCE_LAKE_MINIMUM_INCOMING_RIVERS
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
        confluence_bodies.insert(cell, body.clone());

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

            let lake = spawn_lakes
                .then(|| selection.lakes.get(&cell).cloned())
                .flatten();
            let generated_source_body = lake;
            let source_body = generated_source_body
                .clone()
                .or_else(|| confluence_bodies.get(&cell).cloned());
            let source_body_water_level = source_body.as_ref().map(|body| body.water_level);

            if let Some(body) = generated_source_body
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
            let downstream: DrainageNode = network.node(downstream_cell);
            let downstream_is_destination = network.is_wet_ocean(downstream)
                || selection.lakes.contains_key(&downstream_cell);
            if !downstream_is_destination
                && (!downstream.biome_hydrology.can_generate_river
                    || !selection.channels.contains(&downstream_cell))
            {
                continue;
            }
            let flow = flow_cache.get(&cell).copied().unwrap_or(1);
            let downstream_flow = flow_cache.get(&downstream_cell).copied().unwrap_or(flow);
            let source_water_level = source_body_water_level;
            let downstream_body = selection
                .lakes
                .get(&downstream_cell)
                .cloned()
                .or_else(|| confluence_bodies.get(&downstream_cell).cloned());
            let downstream_water_level = downstream_body.as_ref().map(|body| body.water_level);

            // Every incoming edge ends at the same authoritative drainage node
            // that starts the outgoing edge. Never aim tributaries at a straight
            // chord while the downstream channel actually meanders.
            let waterfall = add_curved_river_edge(
                &mut graph,
                RiverEdgeSpec {
                    region_coord: coord,
                    source_cell: cell,
                    source,
                    downstream,
                    source_water_level,
                    downstream_water_level,
                    source_water_body: source_body,
                    downstream_water_body: downstream_body,
                    flow,
                    downstream_flow,
                    seed,
                    sea_level,
                    ocean_threshold,
                    ocean_weight: network.ocean_weight(),
                },
                |position| network.surface_sample_at(position),
            );

            if let Some(pool) = spawn_lakes
                .then_some(waterfall)
                .flatten()
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

