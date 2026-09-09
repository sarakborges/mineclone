use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct FeatureNode {
    pub position: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct FeatureEdge {
    pub from: usize,
    pub to: usize,
    pub start_radius: f32,
    pub end_radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct FeatureGraphSample {
    pub edge_index: usize,
    pub progress: f32,
    pub distance: f32,
    pub radius: f32,
    pub strength: f32,
}

#[derive(Clone, Debug, Default)]
pub struct FeatureGraph {
    nodes: Vec<FeatureNode>,
    edges: Vec<FeatureEdge>,
}

impl FeatureGraph {
    pub fn nodes(&self) -> &[FeatureNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[FeatureEdge] {
        &self.edges
    }

    pub fn add_node(&mut self, position: Vec3) -> usize {
        let index = self.nodes.len();
        self.nodes.push(FeatureNode { position });
        index
    }

    pub fn add_edge(
        &mut self,
        from: usize,
        to: usize,
        start_radius: f32,
        end_radius: f32,
    ) {
        assert!(from < self.nodes.len(), "feature edge source node is missing");
        assert!(to < self.nodes.len(), "feature edge target node is missing");
        assert!(start_radius > 0.0, "feature edge start radius must be positive");
        assert!(end_radius > 0.0, "feature edge end radius must be positive");

        self.edges.push(FeatureEdge {
            from,
            to,
            start_radius,
            end_radius,
        });
    }

    pub fn sample(&self, position: Vec3) -> Option<FeatureGraphSample> {
        let mut strongest: Option<FeatureGraphSample> = None;

        for (edge_index, edge) in self.edges.iter().enumerate() {
            let from = self.nodes[edge.from].position;
            let to = self.nodes[edge.to].position;
            let segment = to - from;
            let length_squared = segment.x * segment.x
                + segment.y * segment.y
                + segment.z * segment.z;

            if length_squared <= f32::EPSILON {
                continue;
            }

            let relative = position - from;
            let dot = relative.x * segment.x
                + relative.y * segment.y
                + relative.z * segment.z;
            let progress = (dot / length_squared).clamp(0.0, 1.0);
            let closest = from + segment * progress;
            let delta = position - closest;
            let distance = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
            let radius = edge.start_radius + (edge.end_radius - edge.start_radius) * progress;
            let strength = 1.0 - (distance / radius).clamp(0.0, 1.0);

            if strength <= 0.0 {
                continue;
            }

            let candidate = FeatureGraphSample {
                edge_index,
                progress,
                distance,
                radius,
                strength,
            };
            let should_replace = match strongest.as_ref() {
                Some(current) => candidate.strength > current.strength,
                None => true,
            };

            if should_replace {
                strongest = Some(candidate);
            }
        }

        strongest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_strength_inside_feature_edge_radius() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::ZERO);
        let to = graph.add_node(Vec3::new(10.0, 0.0, 0.0));
        graph.add_edge(from, to, 2.0, 2.0);

        let center = graph.sample(Vec3::new(5.0, 0.0, 0.0)).unwrap();
        let edge = graph.sample(Vec3::new(5.0, 1.0, 0.0)).unwrap();

        assert_eq!(center.strength, 1.0);
        assert!(edge.strength > 0.0 && edge.strength < 1.0);
        assert!(graph.sample(Vec3::new(5.0, 3.0, 0.0)).is_none());
    }
}
