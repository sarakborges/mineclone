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
    pub strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct FeatureGraphHorizontalSample {
    pub height: f32,
    pub strength: f32,
}

#[derive(Clone, Debug, Default)]
pub struct FeatureGraph {
    nodes: Vec<FeatureNode>,
    edges: Vec<FeatureEdge>,
}

impl FeatureGraph {
    #[cfg(test)]
    pub fn nodes(&self) -> &[FeatureNode] {
        &self.nodes
    }

    #[cfg(test)]
    pub fn edges(&self) -> &[FeatureEdge] {
        &self.edges
    }

    pub fn add_node(&mut self, position: Vec3) -> usize {
        let index = self.nodes.len();
        self.nodes.push(FeatureNode { position });
        index
    }

    pub fn add_edge(&mut self, from: usize, to: usize, start_radius: f32, end_radius: f32) {
        assert!(
            from < self.nodes.len(),
            "feature edge source node is missing"
        );
        assert!(to < self.nodes.len(), "feature edge target node is missing");
        assert!(
            start_radius > 0.0,
            "feature edge start radius must be positive"
        );
        assert!(end_radius > 0.0, "feature edge end radius must be positive");

        self.edges.push(FeatureEdge {
            from,
            to,
            start_radius,
            end_radius,
        });
    }

    #[cfg(test)]
    pub fn nearest_node(&self, position: Vec3) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                left.position
                    .distance_squared(position)
                    .total_cmp(&right.position.distance_squared(position))
            })
            .map(|(index, _)| index)
    }

    pub fn sample(&self, position: Vec3) -> Option<FeatureGraphSample> {
        let mut strongest: Option<FeatureGraphSample> = None;

        for edge in &self.edges {
            let from = self.nodes[edge.from].position;
            let to = self.nodes[edge.to].position;
            let segment = to - from;
            let length_squared = segment.length_squared();

            if length_squared <= f32::EPSILON {
                continue;
            }

            let relative = position - from;
            let progress = (relative.dot(segment) / length_squared).clamp(0.0, 1.0);
            let closest = from + segment * progress;
            let distance = position.distance(closest);
            let radius = edge.start_radius + (edge.end_radius - edge.start_radius) * progress;
            let strength = 1.0 - (distance / radius).clamp(0.0, 1.0);

            if strength <= 0.0 {
                continue;
            }

            let candidate = FeatureGraphSample { strength };
            if strongest
                .as_ref()
                .is_none_or(|current| candidate.strength > current.strength)
            {
                strongest = Some(candidate);
            }
        }

        strongest
    }

    pub fn sample_horizontal(&self, position: Vec2) -> Option<FeatureGraphHorizontalSample> {
        let mut strongest: Option<FeatureGraphHorizontalSample> = None;

        for edge in &self.edges {
            let from = self.nodes[edge.from].position;
            let to = self.nodes[edge.to].position;
            let from_horizontal = Vec2::new(from.x, from.z);
            let to_horizontal = Vec2::new(to.x, to.z);
            let segment = to_horizontal - from_horizontal;
            let length_squared = segment.length_squared();

            if length_squared <= f32::EPSILON {
                continue;
            }

            let relative = position - from_horizontal;
            let progress = (relative.dot(segment) / length_squared).clamp(0.0, 1.0);
            let closest = from_horizontal + segment * progress;
            let distance = position.distance(closest);
            let radius = edge.start_radius + (edge.end_radius - edge.start_radius) * progress;
            let strength = 1.0 - (distance / radius).clamp(0.0, 1.0);

            if strength <= 0.0 {
                continue;
            }

            let candidate = FeatureGraphHorizontalSample {
                height: from.y + (to.y - from.y) * progress,
                strength,
            };
            if strongest
                .as_ref()
                .is_none_or(|current| candidate.strength > current.strength)
            {
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

    #[test]
    fn horizontal_sampling_interpolates_feature_height() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::new(0.0, 10.0, 0.0));
        let to = graph.add_node(Vec3::new(10.0, 20.0, 0.0));
        graph.add_edge(from, to, 2.0, 2.0);

        let sample = graph.sample_horizontal(Vec2::new(5.0, 0.0)).unwrap();

        assert_eq!(sample.height, 15.0);
        assert_eq!(sample.strength, 1.0);
    }

    #[test]
    fn nearest_node_is_stable_at_feature_boundaries() {
        let mut graph = FeatureGraph::default();
        graph.add_node(Vec3::new(-10.0, 0.0, 0.0));
        graph.add_node(Vec3::new(10.0, 0.0, 0.0));

        assert_eq!(graph.nearest_node(Vec3::new(-9.0, 0.0, 0.0)), Some(0));
        assert_eq!(graph.nearest_node(Vec3::new(9.0, 0.0, 0.0)), Some(1));
    }
}
