use bevy::prelude::*;

use super::feature_graph::FeatureGraph;

const CONNECTOR_REGION_SIZE: f32 = 128.0;
const NODE_JITTER: f32 = 28.0;
const MIN_TUNNEL_RADIUS: f32 = 3.0;
const MAX_TUNNEL_RADIUS: f32 = 8.0;

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

        for direction in [IVec3::X, IVec3::Z, IVec3::Y] {
            let neighbor_coord = coord + direction;
            let neighbor = graph.add_node(node_position(neighbor_coord, self.seed));
            let edge_hash = region_hash(coord, self.seed ^ direction_hash(direction));
            let start_radius = tunnel_radius(edge_hash);
            let end_radius = tunnel_radius(edge_hash.rotate_left(29));

            graph.add_edge(center, neighbor, start_radius, end_radius);
        }

        CaveConnectivityRegion {
            coord,
            connector_graph: graph,
        }
    }
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

fn direction_hash(direction: IVec3) -> u64 {
    ((direction.x as i64 as u64).wrapping_mul(0x9e37_79b9))
        ^ ((direction.y as i64 as u64).wrapping_mul(0x85eb_ca6b))
        ^ ((direction.z as i64 as u64).wrapping_mul(0xc2b2_ae35))
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
}
