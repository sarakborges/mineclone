mod confluence;
mod curve;
mod terrain;
mod waterfall;

use bevy::prelude::*;

use crate::world::{
    feature_graph::FeatureGraph,
    hydrology::types::{HydrologySurfaceSample, WaterBody},
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
        RIVER_FLOW_FOR_MAX_WIDTH, RIVER_MAXIMUM_RADIUS, RIVER_MINIMUM_FLOW,
        RIVER_MINIMUM_RADIUS,
    },
    drainage::{DrainageNode, surface_sample_is_wet_ocean},
    math::lerp,
};

pub(super) const RIVER_WATER_SURFACE_OFFSET: f32 = 2.0;

#[derive(Clone)]
pub(super) struct RiverEdgeSpec {
    pub(super) region_coord: IVec2,
    pub(super) source_cell: IVec2,
    pub(super) source: DrainageNode,
    pub(super) downstream: DrainageNode,
    pub(super) source_water_level: Option<f32>,
    pub(super) downstream_water_level: Option<f32>,
    pub(super) source_water_body: Option<WaterBody>,
    pub(super) downstream_water_body: Option<WaterBody>,
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
    if let Some(body) = spec.source_water_body.as_ref() {
        clip_path_out_of_water_body(&mut path, body);
    }
    if let Some(body) = spec.downstream_water_body.as_ref() {
        clip_path_into_water_body(&mut path, body);
    }
    if path.points.len() < 2 {
        return None;
    }
    truncate_river_at_ocean_mouth(
        &mut path,
        spec.sea_level,
        &mut surface_sample_at,
    );
    if river_path_crosses_disabled_biome(
        &path.points,
        &mut surface_sample_at,
    ) {
        // Downstream selection validates the direct route. A rejected curved
        // path is therefore a meander excursion, not an invalid drainage edge.
        straighten_horizontal_path(&mut path.points);
    }
    let mut surface_elevation_at = |position| surface_sample_at(position).elevation;
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

fn clip_path_out_of_water_body(path: &mut RiverPath, body: &WaterBody) {
    let Some(segment_index) = path.points.windows(2).position(|segment| {
        point_inside_water_body(segment[0], body) && !point_inside_water_body(segment[1], body)
    }) else {
        if path
            .points
            .first()
            .is_some_and(|point| point_inside_water_body(*point, body))
        {
            path.points.clear();
        }
        return;
    };

    let boundary = water_body_boundary_point(
        path.points[segment_index],
        path.points[segment_index + 1],
        body,
        true,
    );
    let mut clipped = Vec::with_capacity(path.points.len() - segment_index);
    clipped.push(boundary);
    clipped.extend_from_slice(&path.points[segment_index + 1..]);
    path.points = clipped;
    if path.waterfall.is_some_and(|waterfall| {
        point_inside_water_body(waterfall.position, body)
    }) {
        path.waterfall = None;
    }
}

fn clip_path_into_water_body(path: &mut RiverPath, body: &WaterBody) {
    let Some(segment_index) = path.points.windows(2).position(|segment| {
        !point_inside_water_body(segment[0], body) && point_inside_water_body(segment[1], body)
    }) else {
        if path
            .points
            .last()
            .is_some_and(|point| point_inside_water_body(*point, body))
        {
            path.points.clear();
        }
        return;
    };

    let boundary = water_body_boundary_point(
        path.points[segment_index],
        path.points[segment_index + 1],
        body,
        false,
    );
    path.points.truncate(segment_index + 1);
    path.points.push(boundary);
    if path.waterfall.is_some_and(|waterfall| {
        point_inside_water_body(waterfall.position, body)
    }) {
        path.waterfall = None;
    }
}

fn point_inside_water_body(point: Vec3, body: &WaterBody) -> bool {
    body.normalized_horizontal_distance(Vec2::new(point.x, point.z)) <= 1.0
}

fn water_body_boundary_point(
    from: Vec3,
    to: Vec3,
    body: &WaterBody,
    from_inside: bool,
) -> Vec3 {
    const REFINEMENT_STEPS: usize = 10;

    let mut inside_t = if from_inside { 0.0 } else { 1.0 };
    let mut outside_t = if from_inside { 1.0 } else { 0.0 };

    for _ in 0..REFINEMENT_STEPS {
        let midpoint_t = (inside_t + outside_t) * 0.5;
        let midpoint = from.lerp(to, midpoint_t);
        if point_inside_water_body(midpoint, body) {
            inside_t = midpoint_t;
        } else {
            outside_t = midpoint_t;
        }
    }

    let mut boundary = from.lerp(to, (inside_t + outside_t) * 0.5);
    boundary.y = body.water_level;
    boundary
}

fn truncate_river_at_ocean_mouth(
    path: &mut RiverPath,
    sea_level: f32,
    sample_at: &mut impl FnMut(Vec2) -> HydrologySurfaceSample,
) {
    const SAMPLES_PER_SEGMENT: usize = 8;
    const BOUNDARY_REFINEMENT_STEPS: usize = 8;

    if path.points.len() < 2 {
        return;
    }

    let waterfall_index = path.waterfall.map(|waterfall| {
        let target = Vec2::new(waterfall.position.x, waterfall.position.z);
        path.points
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                Vec2::new(left.x, left.z)
                    .distance_squared(target)
                    .total_cmp(&Vec2::new(right.x, right.z).distance_squared(target))
            })
            .map(|(index, _)| index)
            .unwrap_or(0)
    });

    let mut mouth = None;
    for (segment_index, segment) in path.points.windows(2).enumerate() {
        let [from, to] = segment else {
            continue;
        };
        let mut previous_t = 0.0;

        for sample_index in 1..=SAMPLES_PER_SEGMENT {
            let t = sample_index as f32 / SAMPLES_PER_SEGMENT as f32;
            let point = from.lerp(*to, t);
            let sample = sample_at(Vec2::new(point.x, point.z));
            if !surface_sample_is_wet_ocean(sample, sea_level) {
                previous_t = t;
                continue;
            }

            let mut dry_t = previous_t;
            let mut wet_t = t;
            for _ in 0..BOUNDARY_REFINEMENT_STEPS {
                let midpoint_t = (dry_t + wet_t) * 0.5;
                let midpoint = from.lerp(*to, midpoint_t);
                let midpoint_sample = sample_at(Vec2::new(midpoint.x, midpoint.z));
                if surface_sample_is_wet_ocean(midpoint_sample, sea_level) {
                    wet_t = midpoint_t;
                } else {
                    dry_t = midpoint_t;
                }
            }

            let mut endpoint = from.lerp(*to, wet_t);
            endpoint.y = sea_level;
            mouth = Some((segment_index, endpoint));
            break;
        }

        if mouth.is_some() {
            break;
        }
    }

    let Some((segment_index, endpoint)) = mouth else {
        return;
    };

    path.points.truncate(segment_index + 1);
    path.points.push(endpoint);

    if waterfall_index.is_some_and(|index| index > segment_index) {
        path.waterfall = None;
    }
}

fn straighten_horizontal_path(points: &mut [Vec3]) {
    let Some((&first, rest)) = points.split_first() else {
        return;
    };
    let Some(last) = rest.last().copied() else {
        return;
    };
    let count = points.len().saturating_sub(1).max(1);

    for (index, point) in points.iter_mut().enumerate() {
        let t = index as f32 / count as f32;
        point.x = lerp(first.x, last.x, t);
        point.z = lerp(first.z, last.z, t);
    }
}

fn river_path_crosses_disabled_biome(
    points: &[Vec3],
    sample_at: &mut impl FnMut(Vec2) -> HydrologySurfaceSample,
) -> bool {
    const SAMPLES_PER_SEGMENT: usize = 4;

    points.windows(2).any(|segment| {
        (0..=SAMPLES_PER_SEGMENT).any(|index| {
            let t = index as f32 / SAMPLES_PER_SEGMENT as f32;
            let point = segment[0].lerp(segment[1], t);
            let sample = sample_at(Vec2::new(point.x, point.z));
            !sample.biome_hydrology.can_generate_river
                && sample.ocean_weight <= f32::EPSILON
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
    if node.ocean_weight > f32::EPSILON && node.elevation < sea_level {
        sea_level
    } else {
        (node.elevation - RIVER_WATER_SURFACE_OFFSET).max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        content::biome_hydrology::BiomeHydrologyRules,
        world::hydrology::HydrologyRegion,
    };

    fn node(position: Vec2, elevation: f32) -> DrainageNode {
        DrainageNode {
            position,
            elevation,
            ocean_weight: 0.0,
            biome_hydrology: BiomeHydrologyRules::default(),
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
    fn straightening_preserves_heights_and_removes_horizontal_meander() {
        let mut points = [
            Vec3::new(0.0, 12.0, 0.0),
            Vec3::new(5.0, 10.0, 8.0),
            Vec3::new(10.0, 8.0, 0.0),
        ];

        straighten_horizontal_path(&mut points);

        assert_eq!(points[0], Vec3::new(0.0, 12.0, 0.0));
        assert_eq!(points[1], Vec3::new(5.0, 10.0, 0.0));
        assert_eq!(points[2], Vec3::new(10.0, 8.0, 0.0));
    }

    #[test]
    fn river_path_rejects_disabled_surface_biome() {
        let points = [Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0)];
        let mut sample = |position: Vec2| HydrologySurfaceSample {
            elevation: 80.0,
            ocean_weight: 0.0,
            biome_hydrology: BiomeHydrologyRules {
                can_generate_river: position.x < 8.0 || position.x > 12.0,
                ..Default::default()
            },
        };

        assert!(river_path_crosses_disabled_biome(&points, &mut sample));
    }

    #[test]
    fn river_stops_at_first_physical_ocean_water() {
        let mut path = RiverPath {
            points: vec![
                Vec3::new(0.0, 70.0, 0.0),
                Vec3::new(10.0, 68.0, 0.0),
                Vec3::new(20.0, 64.0, 0.0),
            ],
            waterfall: None,
        };
        let mut sample = |position: Vec2| HydrologySurfaceSample {
            elevation: if position.x < 12.0 { 70.0 } else { 40.0 },
            ocean_weight: if position.x < 12.0 { 0.0 } else { 1.0 },
            biome_hydrology: BiomeHydrologyRules::default(),
        };

        truncate_river_at_ocean_mouth(&mut path, 64.0, &mut sample);

        let mouth = path.points.last().expect("river should keep an ocean mouth");
        assert!(mouth.x >= 11.9 && mouth.x <= 12.1);
        assert_eq!(mouth.y, 64.0);
        assert!(path.points.iter().all(|point| point.x <= mouth.x));
    }

    #[test]
    fn disabled_underlying_biome_does_not_block_ocean_destination() {
        let points = [Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0)];
        let mut sample = |_position: Vec2| HydrologySurfaceSample {
            elevation: 60.0,
            ocean_weight: 1.0,
            biome_hydrology: BiomeHydrologyRules {
                can_generate_river: false,
                ..Default::default()
            },
        };

        assert!(!river_path_crosses_disabled_biome(&points, &mut sample));
    }

    #[test]
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
                    source_water_body: None,
                    downstream_water_body: None,
                    flow: 4,
                    downstream_flow: 4,
                    seed: 42,
                    sea_level: 64.0,
                },
                |_| HydrologySurfaceSample {
                    elevation: 120.0,
                    ocean_weight: 0.0,
                    biome_hydrology: BiomeHydrologyRules::default(),
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
        let physical_region = |graph| HydrologyRegion {
            river_graph: graph,
            river_carve_depth: 7.0,
            water_bodies: Vec::new(),
            settings: Default::default(),
        };
        let left = physical_region(left);
        let right = physical_region(right);
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
