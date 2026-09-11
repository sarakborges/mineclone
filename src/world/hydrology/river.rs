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
    lake::lake_for_local_basin,
    math::{cell_hash, hash_signed, hash_unit, lerp, smoothstep},
    spatial::{edge_intersects_region, water_body_intersects_region},
    types::{HydrologySurfaceSample, WaterBody},
};

const RIVER_WATER_SURFACE_OFFSET: f32 = 2.0;
const RIVER_MEANDER_CONTROL_SPACING: f32 = 34.0;
const RIVER_MEANDER_MIN_INTERVALS: usize = 3;
const RIVER_MEANDER_MAX_INTERVALS: usize = 8;
const RIVER_STRAIGHT_SECTION_CHANCE: f32 = 0.28;
const MOUNTAIN_SPRING_MINIMUM_HEIGHT_ABOVE_SEA: f32 = 30.0;
const MOUNTAIN_SPRING_MINIMUM_LOCAL_RELIEF: f32 = 6.0;
const MOUNTAIN_SPRING_CHANCE: f32 = 0.34;
const WATERFALL_MINIMUM_DROP: f32 = 10.0;
const WATERFALL_MINIMUM_SLOPE: f32 = 0.075;
const WATERFALL_CHANCE: f32 = 0.72;
const PLUNGE_POOL_CHANCE: f32 = 0.84;

pub(super) struct RiverSystem {
    pub graph: FeatureGraph,
    pub water_bodies: Vec<WaterBody>,
}

struct RiverSelection {
    channels: HashSet<IVec2>,
    springs: HashSet<IVec2>,
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

struct RiverPath {
    points: Vec<Vec3>,
    waterfall: Option<WaterfallLanding>,
}

#[derive(Clone, Copy)]
struct WaterfallProfile {
    start_t: f32,
    end_t: f32,
    drop: f32,
}

#[derive(Clone, Copy)]
struct WaterfallLanding {
    position: Vec3,
    drop: f32,
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
    let selection = selected_river_sources(&flow_cache, seed, sea_level, network);
    let mut outlet_cache = HashMap::new();

    for dz in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
        for dx in -RIVER_EDGE_MARGIN_CELLS..=RIVER_EDGE_MARGIN_CELLS {
            let cell = coord + IVec2::new(dx, dz);
            let source = network.node(cell);

            if source.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD
                || !drainage_reaches_ocean(cell, network, &mut outlet_cache)
            {
                continue;
            }

            let Some(downstream_cell) = network.downstream_cell(cell) else {
                continue;
            };
            let downstream = network.node(downstream_cell);
            let flow = flow_cache.get(&cell).copied().unwrap_or(1);
            let neighbors = network.neighbor_nodes(cell);
            let spring = selection.springs.contains(&cell).then(|| {
                mountain_spring_body(cell, source, seed, sea_level, water_fluid)
            });
            let lake = if spring.is_none() {
                lake_for_local_basin(cell, source, &neighbors, seed, sea_level, water_fluid)
            } else {
                None
            };
            let channel_selected =
                selection.channels.contains(&cell) || spring.is_some() || lake.is_some();

            if let Some(body) = spring
                .or(lake)
                .filter(|body| water_body_intersects_region(coord, body))
            {
                water_bodies.push(body);
            }

            if !channel_selected {
                continue;
            }

            let downstream_flow = flow_cache.get(&downstream_cell).copied().unwrap_or(flow);
            let waterfall = add_curved_river_edge(
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

            if let Some(pool) = waterfall.and_then(|waterfall| {
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

fn drainage_reaches_ocean<F>(
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

fn selected_river_sources<F>(
    flow_cache: &HashMap<IVec2, u32>,
    seed: u64,
    sea_level: f32,
    network: &mut DrainageNetwork<'_, F>,
) -> RiverSelection
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    let mut channels = HashSet::new();
    let mut springs = HashSet::new();

    for (&cell, &flow) in flow_cache {
        let source = network.node(cell);
        if source.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD
            || !source.biome_hydrology.can_generate_river
        {
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
    RiverSelection { channels, springs }
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

fn mountain_spring_body(
    cell: IVec2,
    source: DrainageNode,
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
) -> WaterBody {
    let hash = cell_hash(cell, seed ^ 0x1f83_d9ab_fb41_bd6b);
    let base_radius = lerp(4.0, 7.5, hash_unit(hash.rotate_left(11)));
    let aspect = lerp(0.82, 1.18, hash_unit(hash.rotate_left(27)));

    WaterBody {
        center: source.position,
        radius: Vec2::new(base_radius * aspect, base_radius * (2.0 - aspect)),
        rotation: hash_unit(hash.rotate_left(41)) * std::f32::consts::TAU,
        shape_seed: hash.rotate_left(7),
        water_level: river_height(source, sea_level),
        carve_depth: lerp(2.5, 4.5, hash_unit(hash.rotate_left(53))),
        fluid_id: water_fluid.to_owned(),
    }
}

fn add_curved_river_edge(graph: &mut FeatureGraph, spec: RiverEdgeSpec) -> Option<WaterfallLanding> {
    let width_multiplier = (spec.source.biome_hydrology.river_width_multiplier
        + spec.downstream.biome_hydrology.river_width_multiplier)
        * 0.5;
    let start_radius = river_radius(spec.flow) * width_multiplier;
    let end_radius = river_radius(spec.downstream_flow.max(spec.flow)) * width_multiplier;

    if start_radius <= f32::EPSILON || end_radius <= f32::EPSILON {
        return None;
    }

    let path = river_path(
        spec.source_cell,
        spec.source,
        spec.downstream,
        spec.seed,
        spec.sea_level,
    );
    let last = path.points.len().saturating_sub(1);

    for index in 0..last {
        let from = path.points[index];
        let to = path.points[index + 1];
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

    path.waterfall
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

fn river_path(
    source_cell: IVec2,
    source: DrainageNode,
    downstream: DrainageNode,
    seed: u64,
    sea_level: f32,
) -> RiverPath {
    let delta = downstream.position - source.position;
    let distance = delta.length();
    let segment_count = ((distance / 10.0).ceil() as usize).clamp(7, 26);
    let direction = delta.normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let lateral_controls = river_lateral_controls(source_cell, seed, distance);
    let start_height = river_height(source, sea_level);
    let raw_end_height = river_height(downstream, sea_level);
    let end_height = raw_end_height
        .min(start_height - RIVER_MINIMUM_WATER_DROP)
        .max(1.0);
    let waterfall_profile = waterfall_profile(
        source_cell,
        seed,
        distance,
        start_height,
        end_height,
    );
    let points = (0..=segment_count)
        .map(|index| {
            let t = index as f32 / segment_count as f32;
            let lateral = sample_lateral_controls(&lateral_controls, t);
            let horizontal = source.position.lerp(downstream.position, t) + perpendicular * lateral;
            let height = river_path_height(t, start_height, end_height, waterfall_profile);

            Vec3::new(horizontal.x, height, horizontal.y)
        })
        .collect::<Vec<_>>();
    let waterfall = waterfall_profile.map(|profile| {
        let landing_index = ((profile.end_t * segment_count as f32).round() as usize)
            .min(segment_count);
        WaterfallLanding {
            position: points[landing_index],
            drop: profile.drop,
        }
    });

    RiverPath { points, waterfall }
}

fn river_path_points(
    source_cell: IVec2,
    source: DrainageNode,
    downstream: DrainageNode,
    seed: u64,
    sea_level: f32,
) -> Vec<Vec3> {
    river_path(source_cell, source, downstream, seed, sea_level).points
}

fn waterfall_profile(
    source_cell: IVec2,
    seed: u64,
    distance: f32,
    start_height: f32,
    end_height: f32,
) -> Option<WaterfallProfile> {
    let drop = start_height - end_height;
    if distance <= f32::EPSILON
        || drop < WATERFALL_MINIMUM_DROP
        || drop / distance < WATERFALL_MINIMUM_SLOPE
    {
        return None;
    }

    let hash = cell_hash(source_cell, seed ^ 0x5be0_cd19_137e_2179);
    if hash_unit(hash.rotate_left(9)) >= WATERFALL_CHANCE {
        return None;
    }

    let center = lerp(0.42, 0.68, hash_unit(hash.rotate_left(23)));
    let horizontal_span = lerp(5.0, 10.0, hash_unit(hash.rotate_left(37)));
    let half_width = (horizontal_span / distance * 0.5).clamp(0.025, 0.09);

    Some(WaterfallProfile {
        start_t: (center - half_width).clamp(0.18, 0.78),
        end_t: (center + half_width).clamp(0.22, 0.84),
        drop,
    })
}

fn river_path_height(
    t: f32,
    start_height: f32,
    end_height: f32,
    waterfall: Option<WaterfallProfile>,
) -> f32 {
    let Some(waterfall) = waterfall else {
        return lerp(start_height, end_height, smoothstep(t));
    };

    let approach_height = start_height - waterfall.drop * 0.14;
    let landing_height = end_height + waterfall.drop * 0.10;

    if t <= waterfall.start_t {
        let progress = (t / waterfall.start_t).clamp(0.0, 1.0);
        return lerp(start_height, approach_height, smoothstep(progress));
    }
    if t <= waterfall.end_t {
        let progress = ((t - waterfall.start_t) / (waterfall.end_t - waterfall.start_t))
            .clamp(0.0, 1.0);
        return lerp(approach_height, landing_height, smoothstep(progress));
    }

    let progress = ((t - waterfall.end_t) / (1.0 - waterfall.end_t)).clamp(0.0, 1.0);
    lerp(landing_height, end_height, smoothstep(progress))
}

fn plunge_pool_for_waterfall(
    source_cell: IVec2,
    waterfall: WaterfallLanding,
    seed: u64,
    water_fluid: &str,
) -> Option<WaterBody> {
    let hash = cell_hash(source_cell, seed ^ 0x428a_2f98_d728_ae22);
    if hash_unit(hash.rotate_left(15)) >= PLUNGE_POOL_CHANCE {
        return None;
    }

    let drop_strength = ((waterfall.drop - WATERFALL_MINIMUM_DROP) / 24.0).clamp(0.0, 1.0);
    let base_radius = lerp(
        6.0,
        13.0,
        (drop_strength * 0.7 + hash_unit(hash.rotate_left(31)) * 0.3).clamp(0.0, 1.0),
    );
    let aspect = lerp(0.78, 1.22, hash_unit(hash.rotate_left(47)));

    Some(WaterBody {
        center: Vec2::new(waterfall.position.x, waterfall.position.z),
        radius: Vec2::new(base_radius * aspect, base_radius * (2.0 - aspect)),
        rotation: hash_unit(hash.rotate_left(5)) * std::f32::consts::TAU,
        shape_seed: hash.rotate_left(39),
        water_level: waterfall.position.y + 0.35,
        carve_depth: lerp(4.5, 9.5, drop_strength),
        fluid_id: water_fluid.to_owned(),
    })
}

fn river_lateral_controls(source_cell: IVec2, seed: u64, distance: f32) -> Vec<f32> {
    let interval_count = ((distance / RIVER_MEANDER_CONTROL_SPACING).ceil() as usize)
        .clamp(RIVER_MEANDER_MIN_INTERVALS, RIVER_MEANDER_MAX_INTERVALS);
    let base_hash = cell_hash(source_cell, seed ^ 0x6a09_e667_f3bc_c909);
    let amplitude = (distance * lerp(0.10, 0.28, hash_unit(base_hash))).clamp(7.0, 40.0);
    let mut controls = Vec::with_capacity(interval_count + 1);

    controls.push(0.0);

    for index in 1..interval_count {
        let index_seed = seed
            ^ 0xbb67_ae85_84ca_a73b
            ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let control_hash = cell_hash(source_cell, index_seed ^ base_hash.rotate_left(17));
        let straight_section =
            hash_unit(control_hash.rotate_left(13)) < RIVER_STRAIGHT_SECTION_CHANCE;
        let strength = if straight_section {
            lerp(0.05, 0.22, hash_unit(control_hash.rotate_left(29)))
        } else {
            lerp(0.55, 1.0, hash_unit(control_hash.rotate_left(43)))
        };
        let lateral = hash_signed(control_hash.rotate_left(7)) * amplitude * strength;

        controls.push(lateral);
    }

    controls.push(0.0);
    controls
}

fn sample_lateral_controls(controls: &[f32], t: f32) -> f32 {
    let interval_count = controls.len().saturating_sub(1);
    if interval_count == 0 {
        return 0.0;
    }

    let scaled = t.clamp(0.0, 1.0) * interval_count as f32;
    let index = (scaled.floor() as usize).min(interval_count - 1);
    let local_t = (scaled - index as f32).clamp(0.0, 1.0);
    let p0 = controls[index.saturating_sub(1)];
    let p1 = controls[index];
    let p2 = controls[(index + 1).min(interval_count)];
    let p3 = controls[(index + 2).min(interval_count)];

    catmull_rom(p0, p1, p2, p3, local_t)
}

fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;

    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn river_height(node: DrainageNode, sea_level: f32) -> f32 {
    if node.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD {
        sea_level
    } else {
        (node.elevation - RIVER_WATER_SURFACE_OFFSET).max(1.0)
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
        assert_eq!(points.first().unwrap().z, source.position.y);
        assert_eq!(points.last().unwrap().x, downstream.position.x);
        assert_eq!(points.last().unwrap().z, downstream.position.y);
        assert!(
            points[1..points.len() - 1]
                .iter()
                .any(|point| point.z != 0.0)
        );
    }

    #[test]
    fn river_paths_are_deterministic() {
        let source = node(Vec2::ZERO, 100.0);
        let downstream = node(Vec2::new(128.0, 0.0), 92.0);

        assert_eq!(
            river_path_points(IVec2::new(4, -7), source, downstream, 42, 64.0),
            river_path_points(IVec2::new(4, -7), source, downstream, 42, 64.0),
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

    #[test]
    fn steep_river_drop_can_form_a_waterfall_profile() {
        let profile = waterfall_profile(IVec2::ZERO, 42, 96.0, 110.0, 86.0);

        if let Some(profile) = profile {
            let before = river_path_height(profile.start_t, 110.0, 86.0, Some(profile));
            let after = river_path_height(profile.end_t, 110.0, 86.0, Some(profile));
            assert!(before - after > 110.0 - 86.0 - 8.0);
        }
    }

    #[test]
    fn river_surface_stays_two_blocks_below_land_sample() {
        let source = node(Vec2::ZERO, 80.0);

        assert_eq!(river_height(source, 64.0), 78.0);
    }
}
