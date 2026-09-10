use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use super::{
    constants::{
        OCEAN_CONTINENTALNESS_THRESHOLD, RIVER_BASIN_ESCAPE_RADIUS_CELLS, RIVER_EDGE_MARGIN_CELLS,
        RIVER_FLOW_FOR_MAX_WIDTH, RIVER_FLOW_SEARCH_RADIUS, RIVER_FLOW_TRACE_STEPS,
        RIVER_MAXIMUM_RADIUS, RIVER_MINIMUM_FLOW, RIVER_MINIMUM_RADIUS, RIVER_MINIMUM_WATER_DROP,
    },
    drainage::{DrainageNetwork, DrainageNode},
    lake::{lake_for_local_basin, terminal_lake_for_local_basin},
    math::{cell_hash, hash_signed, hash_unit, lerp, smoothstep},
    spatial::{edge_intersects_region, water_body_intersects_region},
    types::{HydrologySurfaceSample, WaterBody},
};

const MAX_INCOMING_CHANNELS_PER_CONFLUENCE: usize = 2;

pub(super) struct RiverSystem {
    pub graph: FeatureGraph,
    pub water_bodies: Vec<WaterBody>,
}

#[derive(Clone, Copy)]
struct RiverEdgeSpec {
    region_coord: IVec2,
    source_cell: IVec2,
    source: DrainageNode,
    downstream: DrainageNode,
    flow: u32,
    downstream_flow: u32,
    seed: u64,
    sea_level: f32,
}

pub(super) fn build_river_system<F>(
    coord: IVec2,
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
    network: &mut DrainageNetwork<'_, F>,
) -> RiverSystem
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut graph = FeatureGraph::default();
    let mut water_bodies = Vec::new();
    let flow_cache = build_flow_cache(coord, network);
    let selected_sources = selected_river_sources(&flow_cache, network);

    for dz in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
        for dx in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
            let cell = coord + IVec2::new(dx, dz);
            let source = network.node(cell);

            if source.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
                continue;
            }

            let flow = flow_cache.get(&cell).copied().unwrap_or(1);
            let downstream_cell = network.downstream_cell(cell);

            if downstream_cell.is_some() {
                let neighbors = network.neighbor_nodes(cell);
                if let Some(lake) = lake_for_local_basin(
                    cell,
                    source,
                    &neighbors,
                    seed,
                    sea_level,
                    water_fluid,
                )
                .filter(|lake| water_body_intersects_region(coord, lake))
                {
                    water_bodies.push(lake);
                }
            }

            if let Some(downstream_cell) = downstream_cell {
                if !source.biome_hydrology.can_generate_river
                    || flow < RIVER_MINIMUM_FLOW
                    || !selected_sources.contains(&cell)
                {
                    continue;
                }

                let downstream = network.node(downstream_cell);
                if !downstream.biome_hydrology.can_generate_river {
                    continue;
                }

                let downstream_flow = flow_cache.get(&downstream_cell).copied().unwrap_or(flow);

                add_curved_river_edge(
                    &mut graph,
                    RiverEdgeSpec {
                        region_coord: coord,
                        source_cell: cell,
                        source,
                        downstream,
                        flow,
                        downstream_flow,
                        seed,
                        sea_level,
                    },
                );
                continue;
            }

            let neighbors = network.neighbor_nodes(cell);
            let lake = if flow >= RIVER_MINIMUM_FLOW {
                terminal_lake_for_local_basin(
                    cell,
                    source,
                    &neighbors,
                    seed,
                    sea_level,
                    water_fluid,
                )
            } else {
                lake_for_local_basin(cell, source, &neighbors, seed, sea_level, water_fluid)
            };

            if let Some(lake) = lake.filter(|lake| water_body_intersects_region(coord, lake)) {
                water_bodies.push(lake);
            }
        }
    }

    RiverSystem {
        graph,
        water_bodies,
    }
}

fn build_flow_cache<F>(coord: IVec2, network: &mut DrainageNetwork<'_, F>) -> HashMap<IVec2, u32>
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
                if node.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD
                    || !node.biome_hydrology.can_generate_river
                {
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

fn selected_river_sources<F>(
    flow_cache: &HashMap<IVec2, u32>,
    network: &mut DrainageNetwork<'_, F>,
) -> HashSet<IVec2>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut incoming = HashMap::<IVec2, Vec<(IVec2, u32)>>::new();

    for (&cell, &flow) in flow_cache {
        if flow < RIVER_MINIMUM_FLOW {
            continue;
        }

        let source = network.node(cell);
        if source.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD
            || !source.biome_hydrology.can_generate_river
        {
            continue;
        }

        let Some(downstream_cell) = network.downstream_cell(cell) else {
            continue;
        };
        let downstream = network.node(downstream_cell);
        if !downstream.biome_hydrology.can_generate_river {
            continue;
        }

        incoming
            .entry(downstream_cell)
            .or_default()
            .push((cell, flow));
    }

    let mut selected = HashSet::new();

    for channels in incoming.values_mut() {
        channels.sort_by(|(left_cell, left_flow), (right_cell, right_flow)| {
            right_flow
                .cmp(left_flow)
                .then_with(|| left_cell.x.cmp(&right_cell.x))
                .then_with(|| left_cell.y.cmp(&right_cell.y))
        });

        selected.extend(
            channels
                .iter()
                .take(MAX_INCOMING_CHANNELS_PER_CONFLUENCE)
                .map(|(cell, _)| *cell),
        );
    }

    selected
}

fn add_curved_river_edge(graph: &mut FeatureGraph, spec: RiverEdgeSpec) {
    let width_multiplier = (spec.source.biome_hydrology.river_width_multiplier
        + spec.downstream.biome_hydrology.river_width_multiplier)
        * 0.5;
    let start_radius = river_radius(spec.flow) * width_multiplier;
    let end_radius = river_radius(spec.downstream_flow.max(spec.flow)) * width_multiplier;

    if start_radius <= f32::EPSILON || end_radius <= f32::EPSILON {
        return;
    }

    let points = river_path_points(
        spec.source_cell,
        spec.source,
        spec.downstream,
        spec.seed,
        spec.sea_level,
    );
    let last = points.len().saturating_sub(1);

    for index in 0..last {
        let from = points[index];
        let to = points[index + 1];
        if !edge_intersects_region(
            spec.region_coord,
            Vec2::new(from.x, from.z),
            Vec2::new(to.x, to.z),
        ) {
            continue;
        }

        let from_t = index as f32 / last as f32;
        let to_t = (index + 1) as f32 / last as f32;
        let from_radius = lerp(start_radius, end_radius, from_t);
        let to_radius = lerp(start_radius, end_radius, to_t);
        let from_node = graph.add_node(from);
        let to_node = graph.add_node(to);
        graph.add_edge(from_node, to_node, from_radius, to_radius);
    }
}

fn river_radius(flow: u32) -> f32 {
    let minimum = RIVER_MINIMUM_FLOW as f32;
    let normalized = if flow as f32 <= minimum {
        0.0
    } else {
        ((flow as f32 / minimum).ln() / (RIVER_FLOW_FOR_MAX_WIDTH / minimum).ln()).clamp(0.0, 1.0)
    };

    lerp(RIVER_MINIMUM_RADIUS, RIVER_MAXIMUM_RADIUS, normalized)
}

fn river_path_points(
    source_cell: IVec2,
    source: DrainageNode,
    downstream: DrainageNode,
    seed: u64,
    sea_level: f32,
) -> Vec<Vec3> {
    let delta = downstream.position - source.position;
    let distance = delta.length();
    let segment_count = ((distance / 16.0).ceil() as usize).clamp(5, 18);
    let direction = delta.normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let hash = cell_hash(source_cell, seed ^ 0x6a09_e667_f3bc_c909);
    let amplitude = (distance * lerp(0.08, 0.20, hash_unit(hash))).clamp(6.0, 26.0);
    let phase = hash_unit(hash.rotate_left(23)) * std::f32::consts::TAU;
    let secondary = hash_signed(hash.rotate_left(41));
    let start_height = river_height(source, sea_level);
    let raw_end_height = river_height(downstream, sea_level);
    let end_height = raw_end_height
        .min(start_height - RIVER_MINIMUM_WATER_DROP)
        .max(1.0);

    (0..=segment_count)
        .map(|index| {
            let t = index as f32 / segment_count as f32;
            let envelope = (std::f32::consts::PI * t).sin();
            let broad = (phase + t * std::f32::consts::TAU * 0.72).sin();
            let detail = (phase * 0.5 + t * std::f32::consts::TAU * 1.65).sin();
            let lateral = (broad * 0.72 + detail * 0.28 * secondary) * amplitude * envelope;
            let horizontal = source.position.lerp(downstream.position, t) + perpendicular * lateral;
            let height = lerp(start_height, end_height, smoothstep(t));

            Vec3::new(horizontal.x, height, horizontal.y)
        })
        .collect()
}

fn river_height(node: DrainageNode, sea_level: f32) -> f32 {
    if node.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
        sea_level
    } else {
        (node.elevation - 0.65).max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::biome_hydrology::BiomeHydrology;

    fn node(position: Vec2, elevation: f32) -> DrainageNode {
        DrainageNode {
            position,
            elevation,
            continentalness: 0.8,
            biome_hydrology: BiomeHydrology::default(),
        }
    }

    #[test]
    fn river_width_increases_with_flow() {
        assert!(river_radius(RIVER_MINIMUM_FLOW * 3) > river_radius(RIVER_MINIMUM_FLOW));
    }

    #[test]
    fn river_paths_meander_between_exact_endpoints() {
        let source = node(Vec2::ZERO, 100.0);
        let downstream = node(Vec2::new(128.0, 0.0), 92.0);
        let points = river_path_points(IVec2::ZERO, source, downstream, 42, 64.0);

        assert_eq!(points.first().unwrap().x, source.position.x);
        assert_eq!(points.last().unwrap().x, downstream.position.x);
        assert!(
            points[1..points.len() - 1]
                .iter()
                .any(|point| point.z != 0.0)
        );
    }

    #[test]
    fn river_surface_drops_downstream() {
        let source = node(Vec2::ZERO, 80.0);
        let downstream = node(Vec2::new(128.0, 0.0), 79.9);
        let points = river_path_points(IVec2::ZERO, source, downstream, 42, 64.0);
        let start = points.first().unwrap().y;
        let end = points.last().unwrap().y;

        assert!(start - end >= RIVER_MINIMUM_WATER_DROP);
    }
}
