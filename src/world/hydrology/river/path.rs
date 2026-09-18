mod confluence;
mod curve;
mod terrain;
mod waterfall;

use bevy::prelude::*;

use crate::world::{
    feature_graph::FeatureGraph,
    hydrology::types::HydrologySurfaceSample,
};

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
    pub(super) ocean_threshold: f32,
}

pub(super) struct RiverPath {
    pub(super) points: Vec<Vec3>,
    pub(super) waterfall: Option<WaterfallLanding>,
}

pub(super) fn add_curved_river_edge<F>(
    graph: &mut FeatureGraph,
    spec: RiverEdgeSpec,
    mut surface_sample_at: F,
) -> Option<WaterfallLanding>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
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
    if river_path_crosses_disabled_biome(
        &path.points,
        spec.ocean_threshold,
        &mut surface_sample_at,
    ) {
        return None;
    }
    constrain_river_path_to_terrain(
        &mut path.points,
        start_radius,
        end_radius,
        |position| surface_sample_at(position).elevation,
    );
    align_waterfall_landing_to_path(&mut path);
    add_path_to_graph(graph, spec.region_coord, &mut path, start_radius, end_radius);

    path.waterfall
}

fn river_path_crosses_disabled_biome(
    points: &[Vec3],
    ocean_threshold: f32,
    sample_at: &mut impl FnMut(Vec2) -> HydrologySurfaceSample,
) -> bool {
    const SAMPLES_PER_SEGMENT: usize = 4;

    points.windows(2).any(|segment| {
        (0..=SAMPLES_PER_SEGMENT).any(|index| {
            let t = index as f32 / SAMPLES_PER_SEGMENT as f32;
            let point = segment[0].lerp(segment[1], t);
            let sample = sample_at(Vec2::new(point.x, point.z));
            !sample.biome_hydrology.can_generate_river
                && sample.continentalness > ocean_threshold
        })
    })
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
    use crate::{
        content::biome_hydrology::BiomeHydrology,
        world::hydrology::HydrologyRegion,
    };

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
    #[test]
    fn river_path_rejects_disabled_surface_biome() {
        let points = [Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0)];
        let mut sample = |position: Vec2| HydrologySurfaceSample {
            elevation: 80.0,
            continentalness: 0.8,
            biome_hydrology: BiomeHydrology {
                can_generate_river: position.x < 8.0 || position.x > 12.0,
                ..Default::default()
            },
        };

        assert!(river_path_crosses_disabled_biome(
            &points,
            0.45,
            &mut sample
        ));
    }

    #[test]
    fn disabled_underlying_biome_does_not_block_ocean_destination() {
        let points = [Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0)];
        let mut sample = |_position: Vec2| HydrologySurfaceSample {
            elevation: 60.0,
            continentalness: 0.2,
            biome_hydrology: BiomeHydrology {
                can_generate_river: false,
                ..Default::default()
            },
        };

        assert!(!river_path_crosses_disabled_biome(
            &points,
            0.45,
            &mut sample
        ));
    }

    fn neighboring_regions_reproduce_identical_water_height_at_the_same_river_crossing() {
        let source = node(Vec2::new(64.0, 64.0), 100.0);
        let downstream = node(Vec2::new(192.0, 64.0), 90.0);
        let path = river_path(IVec2::ZERO, source, downstream, 42, 64.0, None, None);
        let crossing = path.points.windows(2).find_map(|segment| {
            let [from, to] = segment else { return None };
            if from.x > 128.0 || to.x < 128.0 {
                return None;
            }
            let t = (128.0 - from.x) / (to.x - from.x);
            Some(Vec2::new(128.0, lerp(from.z, to.z, t)))
        }).expect("the river must cross the region boundary");

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
                    ocean_threshold: 0.45,
                },
                |_| HydrologySurfaceSample {
                    elevation: 120.0,
                    continentalness: 0.8,
                    biome_hydrology: BiomeHydrology::default(),
                },
            );
        }

        let left_sample = left.sample_horizontal(crossing).unwrap();
        let right_sample = right.sample_horizontal(crossing).unwrap();
        assert_eq!(left_sample.height, right_sample.height);
        assert_eq!(left_sample.strength, right_sample.strength);
        assert_eq!(left_sample.strength, 1.0);

        // The graph alone is not enough: a seam may share path height yet lose
        // physical water or carving when the two regions select their sources.
        let physical_region = |coord, graph| HydrologyRegion {
            seed: 42,
            coord,
            river_graph: graph,
            river_carve_depth: 7.0,
            water_bodies: Vec::new(),
            sea_level: 64.0,
            settings: Default::default(),
            ocean_weight: 0.0,
            macro_samples: Vec::new(),
        };
        let left = physical_region(IVec2::ZERO, left);
        let right = physical_region(IVec2::X, right);
        let original_surface = 120.0;
        for x in [127.5, 128.0, 128.5] {
            let point = Vec2::new(x, crossing.y);
            let left_water = left.supported_water_at(point, original_surface).unwrap();
            let right_water = right.supported_water_at(point, original_surface).unwrap();
            assert_eq!(left_water.water_level, right_water.water_level);
            assert_eq!(left_water.bed_level, right_water.bed_level);
            let y = left_water.water_level - 0.5;
            let left_delta = left.density_deltas_for_column::<1>(point, y, original_surface);
            let right_delta = right.density_deltas_for_column::<1>(point, y, original_surface);
            assert_eq!(left_delta, right_delta);
            assert!(left_delta[0] < 0.0);
        }
    }
}
