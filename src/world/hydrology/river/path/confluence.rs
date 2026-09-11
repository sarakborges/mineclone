use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use super::RiverPath;
use super::super::super::{math::lerp, spatial::edge_intersects_region};

const RIVER_CONFLUENCE_UPHILL_TOLERANCE: f32 = 1.0;

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
        if !edge_intersects_region(
            region_coord,
            Vec2::new(from.x, from.z),
            Vec2::new(to.x, to.z),
        ) {
            continue;
        }

        let from_t = index as f32 / last as f32;
        let to_t = (index + 1) as f32 / last as f32;
        let from_radius = lerp(start_radius, end_radius, from_t);
        let to_radius = lerp(start_radius, end_radius, to_t);
        let from_horizontal = Vec2::new(from.x, from.z);
        let to_horizontal = Vec2::new(to.x, to.z);

        if let Some(intersection) = graph.first_horizontal_intersection(from_horizontal, to_horizontal)
        {
            let path_height = lerp(from.y, to.y, intersection.progress);
            if intersection.position.y <= path_height + RIVER_CONFLUENCE_UPHILL_TOLERANCE {
                let confluence = Vec3::new(
                    intersection.position.x,
                    path_height.min(intersection.position.y),
                    intersection.position.z,
                );
                let confluence_radius = lerp(from_radius, to_radius, intersection.progress);
                let from_node = graph.add_node(from);
                let to_node = graph.add_node(confluence);
                graph.add_edge(from_node, to_node, from_radius, confluence_radius);
                path.waterfall = None;
                break;
            }
        }

        let from_node = graph.add_node(from);
        let to_node = graph.add_node(to);
        graph.add_edge(from_node, to_node, from_radius, to_radius);
    }
}
