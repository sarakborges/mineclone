use std::collections::HashSet;

use bevy::prelude::*;

use crate::world::{
    deterministic::{compare_vec3, hash_unit, mix_u32_components, sorted_unique_vec3s},
    feature_graph::FeatureGraph,
    generation_region::generation_region_world_bounds,
};

use super::{
    path::{chaotic_connector_points, connector_radius_progress},
    water::UndergroundWaterRegion,
};

const MAX_CONNECTOR_LENGTH: f32 = 384.0;
const MAX_EXTRA_CONNECTIONS_PER_ANCHOR: usize = 2;
const MIN_TUNNEL_RADIUS: f32 = 5.0;
const MAX_TUNNEL_RADIUS: f32 = 11.0;

pub(super) const ANCHOR_SEARCH_MARGIN: f32 = MAX_CONNECTOR_LENGTH;

pub(super) fn build_connector_graph(
    coord: IVec3,
    anchors: &[Vec3],
    underground_anchors: &[Vec3],
    water_source_anchors: &[Vec3],
    seed: u64,
    water_seed: u64,
    underground_water: &mut UndergroundWaterRegion,
) -> FeatureGraph {
    let anchors = normalized_anchors(anchors);
    if anchors.is_empty() {
        return FeatureGraph::default();
    }

    let underground_anchors = normalized_anchors(underground_anchors);
    let water_source_anchors = normalized_anchors(water_source_anchors);
    let mut graph = FeatureGraph::default();
    let mut connected_pairs = HashSet::new();
    let mut candidates = Vec::new();

    for left in 0..anchors.len() {
        for right in left + 1..anchors.len() {
            let distance = anchors[left].distance(anchors[right]);
            if distance > f32::EPSILON && distance <= MAX_CONNECTOR_LENGTH {
                candidates.push((left, right, distance));
            }
        }
    }

    candidates.sort_by(|(left_a, right_a, distance_a), (left_b, right_b, distance_b)| {
        distance_a
            .total_cmp(distance_b)
            .then_with(|| compare_vec3(&anchors[*left_a], &anchors[*left_b]))
            .then_with(|| compare_vec3(&anchors[*right_a], &anchors[*right_b]))
    });

    let mut parents = (0..anchors.len()).collect::<Vec<_>>();
    let mut extra_degree = vec![0_usize; anchors.len()];

    // Build a minimum spanning forest first. Any anchors that can reach each other inside the
    // connector search radius become one continuous tunnel network instead of disconnected
    // nearest-neighbour clusters.
    for &(left, right, _) in &candidates {
        if !union_components(&mut parents, left, right) {
            continue;
        }

        let pair = ordered_pair(left, right);
        connected_pairs.insert(pair);
        add_connector_pair(
            &mut graph,
            underground_water,
            coord,
            &anchors,
            &underground_anchors,
            &water_source_anchors,
            pair,
            seed,
            water_seed,
        );
    }

    // Add a small number of nearby alternate routes after connectivity is guaranteed. This keeps
    // cave systems organic without producing dense webs or arbitrary dead-end branches.
    for &(left, right, _) in &candidates {
        let pair = ordered_pair(left, right);
        if connected_pairs.contains(&pair)
            || extra_degree[left] >= MAX_EXTRA_CONNECTIONS_PER_ANCHOR
            || extra_degree[right] >= MAX_EXTRA_CONNECTIONS_PER_ANCHOR
        {
            continue;
        }

        connected_pairs.insert(pair);
        extra_degree[left] += 1;
        extra_degree[right] += 1;
        add_connector_pair(
            &mut graph,
            underground_water,
            coord,
            &anchors,
            &underground_anchors,
            &water_source_anchors,
            pair,
            seed,
            water_seed,
        );
    }

    graph
}

#[allow(clippy::too_many_arguments)]
fn add_connector_pair(
    graph: &mut FeatureGraph,
    underground_water: &mut UndergroundWaterRegion,
    coord: IVec3,
    anchors: &[Vec3],
    underground_anchors: &[Vec3],
    water_source_anchors: &[Vec3],
    pair: (usize, usize),
    seed: u64,
    water_seed: u64,
) {
    let from = anchors[pair.0];
    let to = anchors[pair.1];
    let carries_water = connection_carries_underground_water(
        from,
        to,
        underground_anchors,
        water_source_anchors,
        water_seed,
    );

    add_connector(
        graph,
        underground_water,
        coord,
        from,
        to,
        seed,
        carries_water,
    );
}

fn connection_carries_underground_water(
    from: Vec3,
    to: Vec3,
    underground_anchors: &[Vec3],
    water_source_anchors: &[Vec3],
    water_seed: u64,
) -> bool {
    let from_underground = contains_anchor(underground_anchors, from);
    let to_underground = contains_anchor(underground_anchors, to);
    let from_water_source = contains_anchor(water_source_anchors, from);
    let to_water_source = contains_anchor(water_source_anchors, to);
    let water_source_connection =
        (from_water_source && to_underground) || (to_water_source && from_underground);

    water_source_connection
        || (from_underground
            && to_underground
            && UndergroundWaterRegion::connection_carries_water(from, to, water_seed))
}

fn add_connector(
    graph: &mut FeatureGraph,
    underground_water: &mut UndergroundWaterRegion,
    coord: IVec3,
    from: Vec3,
    to: Vec3,
    seed: u64,
    carries_water: bool,
) {
    let hash = anchor_pair_hash(from, to, seed);
    let start_radius = tunnel_radius(hash);
    let end_radius = tunnel_radius(hash.rotate_left(29));
    let points = chaotic_connector_points(from, to, hash);
    let (region_minimum, region_maximum) = generation_region_world_bounds(coord);

    for segment in 0..points.len().saturating_sub(1) {
        let from_progress = segment as f32 / (points.len() - 1) as f32;
        let to_progress = (segment + 1) as f32 / (points.len() - 1) as f32;
        let from_radius = connector_radius_progress(start_radius, end_radius, from_progress, hash);
        let to_radius = connector_radius_progress(start_radius, end_radius, to_progress, hash);
        let from = points[segment];
        let to = points[segment + 1];

        if !segment_intersects_region(
            from,
            to,
            from_radius.max(to_radius),
            region_minimum,
            region_maximum,
        ) {
            continue;
        }

        let from_node = graph.add_node(from);
        let to_node = graph.add_node(to);
        graph.add_edge(from_node, to_node, from_radius, to_radius);

        if carries_water {
            underground_water.add_river_segment(from, to, from_radius, to_radius);
        }
    }
}

fn normalized_anchors(anchors: &[Vec3]) -> Vec<Vec3> {
    sorted_unique_vec3s(
        anchors
            .iter()
            .copied()
            .filter(|position| position.y >= 0.0),
    )
}

fn contains_anchor(anchors: &[Vec3], position: Vec3) -> bool {
    anchors.iter().any(|anchor| *anchor == position)
}

fn ordered_pair(left: usize, right: usize) -> (usize, usize) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}

fn find_component(parents: &mut [usize], index: usize) -> usize {
    if parents[index] == index {
        return index;
    }

    let root = find_component(parents, parents[index]);
    parents[index] = root;
    root
}

fn union_components(parents: &mut [usize], left: usize, right: usize) -> bool {
    let left_root = find_component(parents, left);
    let right_root = find_component(parents, right);
    if left_root == right_root {
        return false;
    }

    parents[right_root] = left_root;
    true
}

fn segment_intersects_region(
    from: Vec3,
    to: Vec3,
    radius: f32,
    region_minimum: Vec3,
    region_maximum: Vec3,
) -> bool {
    let margin = Vec3::splat(radius);
    let segment_minimum = from.min(to) - margin;
    let segment_maximum = from.max(to) + margin;

    segment_maximum.x >= region_minimum.x
        && segment_minimum.x <= region_maximum.x
        && segment_maximum.y >= region_minimum.y
        && segment_minimum.y <= region_maximum.y
        && segment_maximum.z >= region_minimum.z
        && segment_minimum.z <= region_maximum.z
}

fn anchor_pair_hash(left: Vec3, right: Vec3, seed: u64) -> u64 {
    let (first, second) = if compare_vec3(&left, &right) != std::cmp::Ordering::Greater {
        (left, right)
    } else {
        (right, left)
    };

    mix_u32_components(
        seed ^ 0x6a09_e667_f3bc_c909,
        [
            first.x.to_bits(),
            first.y.to_bits(),
            first.z.to_bits(),
            second.x.to_bits(),
            second.y.to_bits(),
            second.z.to_bits(),
        ],
    )
}

fn tunnel_radius(hash: u64) -> f32 {
    MIN_TUNNEL_RADIUS + (MAX_TUNNEL_RADIUS - MIN_TUNNEL_RADIUS) * hash_unit(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_pair_radius_is_order_independent() {
        let left = Vec3::new(10.0, 20.0, 30.0);
        let right = Vec3::new(90.0, 40.0, -5.0);
        let seed = 42;

        assert_eq!(
            anchor_pair_hash(left, right, seed),
            anchor_pair_hash(right, left, seed),
        );
    }

    #[test]
    fn explicit_water_source_connection_always_carries_water() {
        let cavern = Vec3::new(32.5, 24.5, 32.5);
        let ocean_opening = Vec3::new(48.5, 52.5, 32.5);

        assert!(connection_carries_underground_water(
            cavern,
            ocean_opening,
            &[cavern],
            &[ocean_opening],
            42,
        ));
    }

    #[test]
    fn candidate_network_connects_reachable_anchor_chain() {
        let anchors = [
            Vec3::new(0.0, 30.0, 0.0),
            Vec3::new(180.0, 30.0, 0.0),
            Vec3::new(360.0, 30.0, 0.0),
        ];
        let mut water = UndergroundWaterRegion::default();
        let graph = build_connector_graph(
            IVec3::ZERO,
            &anchors,
            &anchors,
            &[],
            42,
            84,
            &mut water,
        );

        assert!(graph.edge_count() > 2);
    }
}
