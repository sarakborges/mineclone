mod confluence;
mod curve;
mod terrain;
mod waterfall;

use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use self::{
    confluence::add_path_to_graph,
    curve::river_path,
    terrain::constrain_river_path_to_terrain,
    waterfall::align_waterfall_landing_to_path,
};
pub(super) use self::waterfall::{WATERFALL_MINIMUM_DROP, WaterfallLanding};
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
    add_path_to_graph(graph, spec.region_coord, &mut path, start_radius, end_radius);

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
}
