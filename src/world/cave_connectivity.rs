use bevy::prelude::*;

use super::feature_graph::FeatureGraph;

const MAX_CONNECTOR_LENGTH: f32 = 256.0;
const MIN_TUNNEL_RADIUS: f32 = 3.0;
const MAX_TUNNEL_RADIUS: f32 = 8.0;

#[derive(Clone, Debug, Default)]
pub struct CaveConnectivityRegion {
    pub connector_graph: FeatureGraph,
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
        MAX_CONNECTOR_LENGTH
    }

    pub fn region_from_anchors(&self, coord: IVec3, anchors: &[Vec3]) -> CaveConnectivityRegion {
        if coord.y < 0 || anchors.len() < 2 {
            return CaveConnectivityRegion::default();
        }

        let mut anchors = anchors
            .iter()
            .copied()
            .filter(|position| position.y >= 0.0)
            .collect::<Vec<_>>();
        anchors.sort_by(compare_position);
        anchors.dedup_by(|left, right| *left == *right);

        let mut graph = FeatureGraph::default();
        let nodes = anchors
            .iter()
            .map(|position| graph.add_node(*position))
            .collect::<Vec<_>>();

        for left in 0..anchors.len() {
            for right in (left + 1)..anchors.len() {
                let distance = anchors[left].distance(anchors[right]);

                if distance <= f32::EPSILON || distance > MAX_CONNECTOR_LENGTH {
                    continue;
                }

                let hash = anchor_pair_hash(anchors[left], anchors[right], self.seed);
                let start_radius = tunnel_radius(hash);
                let end_radius = tunnel_radius(hash.rotate_left(29));

                graph.add_edge(nodes[left], nodes[right], start_radius, end_radius);
            }
        }

        CaveConnectivityRegion {
            connector_graph: graph,
        }
    }
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
    fn connector_regions_are_deterministic_and_never_negative_y() {
        let field = CaveConnectivityField::new(42);
        let anchors = [Vec3::new(10.0, 20.0, 10.0), Vec3::new(80.0, 30.0, 20.0)];
        let first = field.region_from_anchors(IVec3::ZERO, &anchors);
        let second = field.region_from_anchors(IVec3::ZERO, &anchors);

        assert_eq!(
            first.connector_graph.nodes().len(),
            second.connector_graph.nodes().len()
        );
        assert_eq!(
            first.connector_graph.edges().len(),
            second.connector_graph.edges().len()
        );

        for (left, right) in first
            .connector_graph
            .nodes()
            .iter()
            .zip(second.connector_graph.nodes())
        {
            assert_eq!(left.position, right.position);
            assert!(left.position.y >= 0.0);
        }
    }

    #[test]
    fn nearby_cavern_anchors_are_connected() {
        let field = CaveConnectivityField::new(42);
        let region = field.region_from_anchors(
            IVec3::ZERO,
            &[Vec3::new(10.0, 20.0, 10.0), Vec3::new(80.0, 30.0, 20.0)],
        );

        assert_eq!(region.connector_graph.nodes().len(), 2);
        assert_eq!(region.connector_graph.edges().len(), 1);
    }

    #[test]
    fn distant_cavern_anchors_do_not_create_unbounded_tunnels() {
        let field = CaveConnectivityField::new(42);
        let region = field.region_from_anchors(
            IVec3::ZERO,
            &[Vec3::ZERO, Vec3::new(MAX_CONNECTOR_LENGTH + 1.0, 0.0, 0.0)],
        );

        assert!(region.connector_graph.edges().is_empty());
    }

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
    fn connector_strength_does_not_depend_on_requesting_region() {
        let field = CaveConnectivityField::new(42);
        let anchors = [Vec3::new(100.0, 40.0, 20.0), Vec3::new(180.0, 40.0, 20.0)];
        let left = field.region_from_anchors(IVec3::ZERO, &anchors);
        let right = field.region_from_anchors(IVec3::X, &anchors);
        let sample_position = Vec3::new(128.0, 40.0, 20.0);
        let left_sample = left.connector_graph.sample(sample_position).unwrap();
        let right_sample = right.connector_graph.sample(sample_position).unwrap();

        assert_eq!(left_sample.strength, right_sample.strength);
    }
}
