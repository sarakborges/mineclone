mod confluence;
mod curve;
mod terrain;
mod waterfall;

use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

pub(super) use self::waterfall::{WATERFALL_MINIMUM_DROP, WaterfallLanding};
use self::{
    confluence::add_path_to_graph, curve::river_path, terrain::constrain_river_path_to_terrain,
    waterfall::align_waterfall_landing_to_path,
};
use super::super::{
    constants::{
        OCEAN_CONTINENTALNESS_THRESHOLD, RIVER_FLOW_FOR_MAX_WIDTH, RIVER_MAXIMUM_RADIUS,
        RIVER_MINIMUM_FLOW, RIVER_MINIMUM_RADIUS,
    },
    drainage::DrainageNode,
    math::lerp,
};

pub(super) const RIVER_WATER_SURFACE_OFFSET: f32 = 2.0;

#[derive(Clone, Copy)]
pub(super) struct RiverEdgeSpec {
    pub(super) region_coord: IVec2,
    pub(super) source_cell: IVec2,
    pub(super) source: DrainageNode,
    pub(super) downstream: DrainageNode,
    pub(super) source_water_level: Option<f32>,
    pub(super) downstream_water_level: Option<f32>,
    pub(super) flow: u32,
    pub(super) downstream_flow: u32,
    pub(super) seed: u64,
    pub(super) sea_level: f32,
}

pub(super) struct RiverPath {
    pub(super) points: Vec<Vec3>,
    pub(super) waterfall: Option<WaterfallLanding>,
}

pub(super) fn add_curved_river_edge<F>(
    graph: &mut FeatureGraph,
    spec: RiverEdgeSpec,
    mut surface_elevation_at: F,
) -> Option<WaterfallLanding>
where
    F: FnMut(Vec2) -> f32,
{
    let width_multiplier = (spec.source.biome_hydrology.river_width_multiplier
        + spec.downstream.biome_hydrology.river_width_multiplier)
        * 0.5;
    let start_radius = river_radius(spec.flow) * width_multiplier;
    let end_radius = river_radius(spec.downstream_flow.max(spec.flow)) * width_multiplier;

    if start_radius <= f32::EPSILON || end_radius <= f32::EPSILON {
        return None;
    }

    let mut path = river_path(
        spec.source_cell,
        spec.source,
        spec.downstream,
        spec.seed,
        spec.sea_level,
        spec.source_water_level,
        spec.downstream_water_level,
    );
    constrain_river_path_to_terrain(
        &mut path.points,
        start_radius,
        end_radius,
        &mut surface_elevation_at,
    );
    align_waterfall_landing_to_path(&mut path);
    add_path_to_graph(
        graph,
        spec.region_coord,
        &mut path,
        start_radius,
        end_radius,
    );

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

pub(super) fn river_height(node: DrainageNode, sea_level: f32) -> f32 {
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
    fn river_surface_stays_two_blocks_below_land_sample() {
        let source = node(Vec2::ZERO, 80.0);

        assert_eq!(river_height(source, 64.0), 78.0);
    }

    #[test]
    fn neighboring_regions_reproduce_identical_water_height_at_the_same_river_crossing() {
        let source = node(Vec2::new(64.0, 64.0), 100.0);
        let downstream = node(Vec2::new(192.0, 64.0), 90.0);
        let path = river_path(IVec2::ZERO, source, downstream, 42, 64.0, None, None);
        let crossing = path
            .points
            .windows(2)
            .find_map(|segment| {
                let [from, to] = segment else { return None };
                if from.x > 128.0 || to.x < 128.0 {
                    return None;
                }
                let t = (128.0 - from.x) / (to.x - from.x);
                Some(Vec2::new(128.0, lerp(from.z, to.z, t)))
            })
            .expect("the river must cross the region boundary");

        let mut left = FeatureGraph::default();
        let mut right = FeatureGraph::default();
        for (coord, graph) in [(IVec2::ZERO, &mut left), (IVec2::X, &mut right)] {
            add_curved_river_edge(
                graph,
                RiverEdgeSpec {
                    region_coord: coord,
                    source_cell: IVec2::ZERO,
                    source,
                    downstream,
                    source_water_level: None,
                    downstream_water_level: None,
                    flow: 4,
                    downstream_flow: 4,
                    seed: 42,
                    sea_level: 64.0,
                },
                |_| 120.0,
            );
        }

        let left_sample = left.sample_horizontal(crossing).unwrap();
        let right_sample = right.sample_horizontal(crossing).unwrap();
        assert_eq!(left_sample.height, right_sample.height);
        assert_eq!(left_sample.strength, right_sample.strength);
        assert_eq!(left_sample.strength, 1.0);
    }
}
