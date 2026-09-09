use bevy::prelude::*;

use super::feature_graph::FeatureGraph;

const CONNECTOR_REGION_SIZE: f32 = 128.0;
const NODE_JITTER: f32 = 28.0;
const MIN_TUNNEL_RADIUS: f32 = 3.0;
const MAX_TUNNEL_RADIUS: f32 = 8.0;
const ANCHOR_TUNNEL_RADIUS: f32 = 6.0;

#[derive(Clone, Debug)]
pub struct CaveConnectivityRegion {
    pub coord: IVec3,
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

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn region_coord(position: Vec3) -> IVec3 {
        IVec3::new(
            (position.x / CONNECTOR_REGION_SIZE).floor() as i32,
            ((position.y / CONNECTOR_REGION_SIZE).floor() as i32).max(0),
            (position.z / CONNECTOR_REGION_SIZE).floor() as i32,
        )
    }

    pub fn region(&self, coord: IVec3) -> CaveConnectivityRegion {
        if coord.y < 0 {
            return CaveConnectivityRegion {
                coord,
                connector_graph: FeatureGraph::default(),
            };
        }

        let mut graph = FeatureGraph::default();
        let center = graph.add_node(node_position(coord, self.seed));

        for direction in [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Z,
            IVec3::NEG_Z,
            IVec3::Y,
            IVec3::NEG_Y,
        ] {
            let neighbor_coord = coord + direction;

            if neighbor_coord.y < 0 {
                continue;
            }

            let neighbor = graph.add_node(node_position(neighbor_coord, self.seed));
            let (center_radius, neighbor_radius) =
                canonical_edge_radii(coord, neighbor_coord, self.seed);

            graph.add_edge(center, neighbor, center_radius, neighbor_radius);
        }

        CaveConnectivityRegion {
            coord,
            connector_graph: graph,
        }
    }

    pub fn region_with_anchors(
        &self,
        base: &CaveConnectivityRegion,
        anchors: &[Vec3],
    ) -> CaveConnectivityRegion {
        if anchors.is_empty() {
            return base.clone();
        }

        let mut connector_graph = base.connector_graph.clone();

        for &anchor in anchors {
            let target_coord = Self::region_coord(anchor);
            let target = node_position(target_coord, self.seed);
            let anchor_node = connector_graph.add_node(anchor);
            let target_node = connector_graph.add_node(target);
            connector_graph.add_edge(
                anchor_node,
                target_node,
                ANCHOR_TUNNEL_RADIUS,
                ANCHOR_TUNNEL_RADIUS,
            );
        }

        CaveConnectivityRegion {
            coord: base.coord,
            connector_graph,
        }
    }
}

fn canonical_edge_radii(from: IVec3, to: IVec3, seed: u64) -> (f32, f32) {
    let edge_hash = canonical_edge_hash(from, to, seed);
    let from_radius = tunnel_radius(edge_hash ^ region_hash(from, seed.rotate_left(11)));
    let to_radius = tunnel_radius(edge_hash ^ region_hash(to, seed.rotate_left(11)));

    (from_radius, to_radius)
}

fn canonical_edge_hash(left: IVec3, right: IVec3, seed: u64) -> u64 {
    let (first, second) = if coord_key(left) <= coord_key(right) {
        (left, right)
    } else {
        (right, left)
    };
    let mut hash = region_hash(first, seed ^ 0x6a09_e667_f3bc_c909);
    hash ^= region_hash(second, seed.rotate_left(31));
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^ (hash >> 32)
}

fn coord_key(coord: IVec3) -> (i32, i32, i32) {
    (coord.x, coord.y, coord.z)
}

fn node_position(coord: IVec3, seed: u64) -> Vec3 {
    let hash = region_hash(coord, seed);
    let base = Vec3::new(
        (coord.x as f32 + 0.5) * CONNECTOR_REGION_SIZE,
        (coord.y as f32 + 0.5) * CONNECTOR_REGION_SIZE,
        (coord.z as f32 + 0.5) * CONNECTOR_REGION_SIZE,
    );
    let jitter = Vec3::new(
        hash_signed(hash) * NODE_JITTER,
        hash_signed(hash.rotate_left(21)) * NODE_JITTER,
        hash_signed(hash.rotate_left(43)) * NODE_JITTER,
    );
    let mut position = base + jitter;
    position.y = position.y.max(1.0);
    position
}

fn tunnel_radius(hash: u64) -> f32 {
    MIN_TUNNEL_RADIUS + (MAX_TUNNEL_RADIUS - MIN_TUNNEL_RADIUS) * hash_unit(hash)
}

fn region_hash(coord: IVec3, seed: u64) -> u64 {
    let mut hash = seed ^ 0x94d0_49bb_1331_11eb;
    hash ^= (coord.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (coord.y as i64 as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= (coord.z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 31;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

fn hash_signed(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_regions_are_deterministic_and_never_negative_y() {
        let field = CaveConnectivityField::new(42);
        let first = field.region(IVec3::new(3, 0, -2));
        let second = field.region(IVec3::new(3, 0, -2));

        assert_eq!(first.connector_graph.nodes().len(), second.connector_graph.nodes().len());
        assert_eq!(first.connector_graph.edges().len(), second.connector_graph.edges().len());

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
    fn anchors_add_canonical_connections_to_the_region_graph() {
        let field = CaveConnectivityField::new(42);
        let base = field.region(IVec3::ZERO);
        let anchor = Vec3::new(10.0, 20.0, 10.0);
        let first = field.region_with_anchors(&base, &[anchor]);
        let second = field.region_with_anchors(&base, &[anchor]);

        assert_eq!(first.connector_graph.nodes().len(), base.connector_graph.nodes().len() + 2);
        assert_eq!(first.connector_graph.edges().len(), base.connector_graph.edges().len() + 1);
        assert_eq!(
            first.connector_graph.nodes().last().unwrap().position,
            second.connector_graph.nodes().last().unwrap().position,
        );
    }

    #[test]
    fn shared_region_edges_have_matching_endpoint_radii() {
        let seed = 42;
        let left = IVec3::ZERO;
        let right = IVec3::X;
        let (left_radius, right_radius) = canonical_edge_radii(left, right, seed);
        let (right_reverse, left_reverse) = canonical_edge_radii(right, left, seed);

        assert_eq!(left_radius, left_reverse);
        assert_eq!(right_radius, right_reverse);
    }
}
