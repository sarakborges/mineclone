mod connectors;
mod path;
mod water;

use bevy::prelude::*;

use self::{
    connectors::{ANCHOR_SEARCH_MARGIN, build_connector_graph},
    water::{UndergroundWaterRegion, UndergroundWaterSample},
};
use super::feature_graph::FeatureGraph;

#[derive(Clone, Debug, Default)]
pub struct CaveConnectivityRegion {
    pub connector_graph: FeatureGraph,
    underground_water: UndergroundWaterRegion,
}

impl CaveConnectivityRegion {
    pub(crate) fn underground_water_at(
        &self,
        position: Vec3,
    ) -> Option<UndergroundWaterSample> {
        self.underground_water.water_at(position)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CaveConnectivityField {
    seed: u64,
}

impl CaveConnectivityField {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn anchor_search_margin(&self) -> f32 {
        ANCHOR_SEARCH_MARGIN
    }

    #[cfg(test)]
    pub fn region_from_anchors(&self, coord: IVec3, anchors: &[Vec3]) -> CaveConnectivityRegion {
        self.region_from_anchors_with_underground_water(coord, anchors, anchors)
    }

    #[cfg(test)]
    pub(crate) fn region_from_anchors_with_underground_water(
        &self,
        coord: IVec3,
        anchors: &[Vec3],
        underground_anchors: &[Vec3],
    ) -> CaveConnectivityRegion {
        self.region_from_anchors_with_water_sources(coord, anchors, underground_anchors, &[])
    }

    pub(crate) fn region_from_anchors_with_water_sources(
        &self,
        coord: IVec3,
        anchors: &[Vec3],
        underground_anchors: &[Vec3],
        water_source_anchors: &[Vec3],
    ) -> CaveConnectivityRegion {
        if coord.y < 0 || anchors.is_empty() {
            return CaveConnectivityRegion::default();
        }

        let water_seed = self.seed.rotate_left(17) ^ 0xbb67_ae85_84ca_a73b;
        let mut underground_water = UndergroundWaterRegion::from_anchors(underground_anchors, water_seed);
        let connector_graph = build_connector_graph(
            coord,
            anchors,
            underground_anchors,
            water_source_anchors,
            self.seed,
            water_seed,
            &mut underground_water,
        );

        CaveConnectivityRegion {
            connector_graph,
            underground_water,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_regions_are_deterministic_and_never_negative_y() {
        let field = CaveConnectivityField::new(42);
        let anchors = [Vec3::new(10.0, 20.0, 10.0), Vec3::new(80.0, 30.0, 20.0)];
        let first = field.region_from_anchors(IVec3::ZERO, &anchors);
        let second = field.region_from_anchors(IVec3::ZERO, &anchors);

        assert_eq!(
            first.connector_graph.node_positions().count(),
            second.connector_graph.node_positions().count()
        );
        assert_eq!(
            first.connector_graph.edge_count(),
            second.connector_graph.edge_count()
        );

        for (left, right) in first
            .connector_graph
            .node_positions()
            .zip(second.connector_graph.node_positions())
        {
            assert_eq!(left, right);
            assert!(left.y >= 0.0);
        }
    }

    #[test]
    fn nearby_cavern_anchors_are_connected_by_a_path() {
        let field = CaveConnectivityField::new(42);
        let region = field.region_from_anchors(
            IVec3::ZERO,
            &[Vec3::new(10.0, 20.0, 10.0), Vec3::new(80.0, 30.0, 20.0)],
        );

        assert!(region.connector_graph.node_positions().count() > 2);
        assert!(region.connector_graph.edge_count() > 1);
    }

    #[test]
    fn dense_anchor_sets_create_sparse_connector_graphs() {
        let field = CaveConnectivityField::new(42);
        let anchors = (0..12)
            .map(|index| Vec3::new(index as f32 * 18.0, 40.0, 32.0))
            .collect::<Vec<_>>();
        let region = field.region_from_anchors(IVec3::ZERO, &anchors);

        assert!(region.connector_graph.edge_count() < 12 * 11 / 2 * 5);
    }

    #[test]
    fn distant_cavern_anchors_do_not_create_unbounded_tunnels() {
        let field = CaveConnectivityField::new(42);
        let region = field.region_from_anchors(
            IVec3::ZERO,
            &[Vec3::ZERO, Vec3::new(ANCHOR_SEARCH_MARGIN + 1.0, 0.0, 0.0)],
        );

        assert_eq!(region.connector_graph.edge_count(), 0);
    }

    #[test]
    fn connector_strength_does_not_depend_on_requesting_region() {
        let field = CaveConnectivityField::new(42);
        let anchors = [Vec3::new(100.0, 40.0, 20.0), Vec3::new(180.0, 40.0, 20.0)];
        let left = field.region_from_anchors(IVec3::ZERO, &anchors);
        let right = field.region_from_anchors(IVec3::X, &anchors);
        let sample_position = Vec3::new(128.0, 40.0, 20.0);
        let left_sample = left.connector_graph.sample(sample_position);
        let right_sample = right.connector_graph.sample(sample_position);

        assert_eq!(left_sample.is_some(), right_sample.is_some());
        if let (Some(left_sample), Some(right_sample)) = (left_sample, right_sample) {
            assert_eq!(left_sample.strength, right_sample.strength);
        }
    }

    #[test]
    fn surface_entrance_does_not_seed_underground_water() {
        let field = CaveConnectivityField::new(42);
        let cavern = Vec3::new(32.5, 24.5, 32.5);
        let entrance = Vec3::new(64.5, 80.5, 32.5);
        let region = field.region_from_anchors_with_underground_water(
            IVec3::ZERO,
            &[cavern, entrance],
            &[cavern],
        );

        assert!(region.underground_water_at(entrance).is_none());
    }
}
