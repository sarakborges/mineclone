use std::collections::HashSet;

use bevy::prelude::*;

use crate::world::{
    feature_graph::FeatureGraph,
    generation_region::generation_region_world_bounds,
};

use super::{
    path::{chaotic_connector_points, connector_radius_progress},
    water::UndergroundWaterRegion,
};

const MAX_CONNECTOR_LENGTH: f32 = 256.0;
const MAX_CONNECTIONS_PER_ANCHOR: usize = 3;
const MIN_TUNNEL_RADIUS: f32 = 3.0;
const MAX_TUNNEL_RADIUS: f32 = 8.0;

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

    for left in 0..anchors.len() {
        let mut neighbors = anchors
            .iter()
            .enumerate()
            .filter_map(|(right, position)| {
                if left == right {
                    return None;
                }

                let distance = anchors[left].distance(*position);
                (distance > f32::EPSILON && distance <= MAX_CONNECTOR_LENGTH)
                    .then_some((right, distance))
            })
            .collect::<Vec<_>>();
        neighbors.sort_by(
            |(left_index, left_distance), (right_index, right_distance)| {
                left_distance.total_cmp(right_distance).then_with(|| {
                    compare_position(&anchors[*left_index], &anchors[*right_index])
                })
            },
        );

        for (right, _) in neighbors.into_iter().take(MAX_CONNECTIONS_PER_ANCHOR) {
            let pair = ordered_pair(left, right);
            if !connected_pairs.insert(pair) {
                continue;
            }

            let from = anchors[pair.0];
            let to = anchors[pair.1];
            let carries_water = connection_carries_underground_water(
                from,
                to,
                &underground_anchors,
                &water_source_anchors,
                water_seed,
            );

            add_connector(
                &mut graph,
                underground_water,
                coord,
                from,
                to,
                seed,
                carries_water,
            );
        }
    }

    graph
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
    let mut anchors = anchors
        .iter()
        .copied()
        .filter(|position| position.y >= 0.0)
        .collect::<Vec<_>>();
    anchors.sort_by(compare_position);
    anchors.dedup_by(|left, right| *left == *right);
    anchors
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

fn compare_position(left: &Vec3, right: &Vec3) -> std::cmp::Ordering {
    left.x
        .total_cmp(&right.x)
        .then_with(|| left.y.total_cmp(&right.y))
        .then_with(|| left.z.total_cmp(&right.z))
}

fn anchor_pair_hash(left: Vec3, right: Vec3, seed: u64) -> u64 {
    let (first, second) = if compare_position(&left, &right) != std::cmp::Ordering::Greater {
        (left, right)
    } else {
        (right, left)
    };
    let mut hash = seed ^ 0x6a09_e667_f3bc_c909;

    for component in [
        first.x.to_bits(),
        first.y.to_bits(),
        first.z.to_bits(),
        second.x.to_bits(),
        second.y.to_bits(),
        second.z.to_bits(),
    ] {
        hash ^= component as u64;
        hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 31;
    }

    hash
}

fn tunnel_radius(hash: u64) -> f32 {
    MIN_TUNNEL_RADIUS + (MAX_TUNNEL_RADIUS - MIN_TUNNEL_RADIUS) * hash_unit(hash)
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
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
}
