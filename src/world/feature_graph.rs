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
    minimum: Vec3,
    maximum: Vec3,
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct FeatureGraphHorizontalIntersection {
    pub(crate) position: Vec3,
    pub(crate) progress: f32,
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

        let from_position = self.nodes[from].position;
        let to_position = self.nodes[to].position;
        let margin = Vec3::splat(start_radius.max(end_radius));

        self.edges.push(FeatureEdge {
            from,
            to,
            start_radius,
            end_radius,
            minimum: from_position.min(to_position) - margin,
            maximum: from_position.max(to_position) + margin,
        });
    }

    pub(crate) fn first_horizontal_intersection(
        &self,
        from: Vec2,
        to: Vec2,
    ) -> Option<FeatureGraphHorizontalIntersection> {
        let direction = to - from;
        if direction.length_squared() <= f32::EPSILON {
            return None;
        }

        let mut first: Option<FeatureGraphHorizontalIntersection> = None;

        for edge in &self.edges {
            let edge_from = self.nodes[edge.from].position;
            let edge_to = self.nodes[edge.to].position;
            let edge_from_horizontal = Vec2::new(edge_from.x, edge_from.z);
            let edge_to_horizontal = Vec2::new(edge_to.x, edge_to.z);
            let Some((progress, edge_progress)) = horizontal_segment_intersection_parameters(
                from,
                to,
                edge_from_horizontal,
                edge_to_horizontal,
            ) else {
                continue;
            };

            let horizontal = from + direction * progress;
            let height = edge_from.y + (edge_to.y - edge_from.y) * edge_progress;
            let candidate = FeatureGraphHorizontalIntersection {
                position: Vec3::new(horizontal.x, height, horizontal.y),
                progress,
            };

            if first
                .as_ref()
                .is_none_or(|current| candidate.progress < current.progress)
            {
                first = Some(candidate);
            }
        }

        first
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
            if !edge_contains_position(edge, position) {
                continue;
            }

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
        self.sample_horizontal_with_margin(position, 0.0)
    }

    pub fn sample_horizontal_with_margin(
        &self,
        position: Vec2,
        margin: f32,
    ) -> Option<FeatureGraphHorizontalSample> {
        let margin = margin.max(0.0);
        let mut strongest: Option<FeatureGraphHorizontalSample> = None;

        for edge in &self.edges {
            if position.x < edge.minimum.x - margin
                || position.x > edge.maximum.x + margin
                || position.y < edge.minimum.z - margin
                || position.y > edge.maximum.z + margin
            {
                continue;
            }

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
            let radius = edge.start_radius
                + (edge.end_radius - edge.start_radius) * progress
                + margin;
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

fn horizontal_segment_intersection_parameters(
    from: Vec2,
    to: Vec2,
    edge_from: Vec2,
    edge_to: Vec2,
) -> Option<(f32, f32)> {
    const ENDPOINT_EPSILON: f32 = 0.001;

    let direction = to - from;
    let edge_direction = edge_to - edge_from;
    let denominator = cross_2d(direction, edge_direction);
    if denominator.abs() <= f32::EPSILON {
        return None;
    }

    let delta = edge_from - from;
    let progress = cross_2d(delta, edge_direction) / denominator;
    let edge_progress = cross_2d(delta, direction) / denominator;

    if progress <= ENDPOINT_EPSILON
        || progress >= 1.0 - ENDPOINT_EPSILON
        || edge_progress <= ENDPOINT_EPSILON
        || edge_progress >= 1.0 - ENDPOINT_EPSILON
    {
        return None;
    }

    Some((progress, edge_progress))
}

fn cross_2d(left: Vec2, right: Vec2) -> f32 {
    left.x * right.y - left.y * right.x
}

fn edge_contains_position(edge: &FeatureEdge, position: Vec3) -> bool {
    position.x >= edge.minimum.x
        && position.x <= edge.maximum.x
        && position.y >= edge.minimum.y
        && position.y <= edge.maximum.y
        && position.z >= edge.minimum.z
        && position.z <= edge.maximum.z
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
    fn horizontal_margin_detects_nearby_feature_without_widening_base_sample() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::ZERO);
        let to = graph.add_node(Vec3::new(10.0, 0.0, 0.0));
        graph.add_edge(from, to, 2.0, 2.0);

        let nearby = Vec2::new(5.0, 5.0);
        assert!(graph.sample_horizontal(nearby).is_none());
        assert!(graph.sample_horizontal_with_margin(nearby, 4.0).is_some());
    }

    #[test]
    fn finds_proper_horizontal_segment_crossings() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::new(-5.0, 10.0, 0.0));
        let to = graph.add_node(Vec3::new(5.0, 8.0, 0.0));
        graph.add_edge(from, to, 2.0, 2.0);

        let intersection = graph
            .first_horizontal_intersection(Vec2::new(0.0, -5.0), Vec2::new(0.0, 5.0))
            .unwrap();

        assert!((intersection.progress - 0.5).abs() < 0.001);
        assert!((intersection.position.y - 9.0).abs() < 0.001);
    }

    #[test]
    fn shared_segment_endpoints_are_not_crossings() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::ZERO);
        let to = graph.add_node(Vec3::new(10.0, 0.0, 0.0));
        graph.add_edge(from, to, 2.0, 2.0);

        assert!(
            graph
                .first_horizontal_intersection(Vec2::new(10.0, 0.0), Vec2::new(20.0, 10.0))
                .is_none()
        );
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
