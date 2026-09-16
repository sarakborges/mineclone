use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use super::RiverPath;
use super::super::super::{
    constants::RIVER_BANK_OUTER_NORMALIZED_DISTANCE,
    math::lerp,
    spatial::edge_intersects_region,
};

pub(super) fn add_path_to_graph(
    graph: &mut FeatureGraph,
    region_coord: IVec2,
    path: &mut RiverPath,
    start_radius: f32,
    end_radius: f32,
) {
    let last = path.points.len().saturating_sub(1);

    for index in 0..last {
        let from = path.points[index];
        let to = path.points[index + 1];
        let from_t = index as f32 / last as f32;
        let to_t = (index + 1) as f32 / last as f32;
        let from_radius = lerp(start_radius, end_radius, from_t);
        let to_radius = lerp(start_radius, end_radius, to_t);
        // Region membership must cover the density bank, not only water.
        // Use the actual radii: biome width multipliers can exceed 1.0.
        let bank_margin = from_radius.max(to_radius) * RIVER_BANK_OUTER_NORMALIZED_DISTANCE;
        if !edge_intersects_region(
            region_coord,
            Vec2::new(from.x, from.z),
            Vec2::new(to.x, to.z),
            bank_margin,
        ) {
            continue;
        }

        let from_node = graph.add_node(from);
        let to_node = graph.add_node(to);
        graph.add_edge(from_node, to_node, from_radius, to_radius);
    }
}
