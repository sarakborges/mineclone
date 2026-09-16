use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
struct FeatureNode {
    position: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct FeatureEdge {
    from: usize,
    to: usize,
    start_radius: f32,
    end_radius: f32,
    minimum: Vec3,
    maximum: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FeatureGraphSample {
    pub(crate) strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FeatureGraphHorizontalSample {
    pub(crate) height: f32,
    pub(crate) strength: f32,
    pub(crate) normalized_distance: f32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FeatureGraph {
    nodes: Vec<FeatureNode>,
    edges: Vec<FeatureEdge>,
}

impl FeatureGraph {
    #[cfg(test)]
    pub(crate) fn node_positions(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.nodes.iter().map(|node| node.position)
    }

    #[cfg(test)]
    pub(crate) fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub(crate) fn add_node(&mut self, position: Vec3) -> usize {
        let index = self.nodes.len();
        self.nodes.push(FeatureNode { position });
        index
    }

    pub(crate) fn add_edge(&mut self, from: usize, to: usize, start_radius: f32, end_radius: f32) {
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

    #[cfg(test)]
    fn nearest_node(&self, position: Vec3) -> Option<usize> {
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

    pub(crate) fn sample(&self, position: Vec3) -> Option<FeatureGraphSample> {
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

    pub(crate) fn sample_horizontal(&self, position: Vec2) -> Option<FeatureGraphHorizontalSample> {
        self.sample_horizontal_with_margin(position, 0.0)
    }

    pub(crate) fn sample_horizontal_with_margin(
        &self,
        position: Vec2,
        margin: f32,
    ) -> Option<FeatureGraphHorizontalSample> {
        self.sample_horizontal_expanded(position, margin.max(0.0), 1.0)
    }

    // River bank grading scales with the base radius of EACH edge, not a
    // fixed global extra width. Biomes may multiply widths above the default.
    pub(crate) fn sample_horizontal_with_radius_multiplier(
        &self,
        position: Vec2,
        radius_multiplier: f32,
    ) -> Option<FeatureGraphHorizontalSample> {
        self.sample_horizontal_expanded(position, 0.0, radius_multiplier.max(1.0))
    }

    fn sample_horizontal_expanded(
        &self,
        position: Vec2,
        margin: f32,
        radius_multiplier: f32,
    ) -> Option<FeatureGraphHorizontalSample> {
        let mut strongest: Option<FeatureGraphHorizontalSample> = None;

        for edge in &self.edges {
            // The stored bounding box already includes one maximum edge
            // radius; expand only by the remainder of the requested footprint.
            let extra = margin + edge.start_radius.max(edge.end_radius) * (radius_multiplier - 1.0);
            if position.x < edge.minimum.x - extra
                || position.x > edge.maximum.x + extra
                || position.y < edge.minimum.z - extra
                || position.y > edge.maximum.z + extra
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
            let base_radius = edge.start_radius + (edge.end_radius - edge.start_radius) * progress;
            let radius = base_radius * radius_multiplier + margin;
            let strength = 1.0 - (distance / radius).clamp(0.0, 1.0);

            if strength <= 0.0 {
                continue;
            }

            let candidate = FeatureGraphHorizontalSample {
                height: from.y + (to.y - from.y) * progress,
                strength,
                normalized_distance: distance / base_radius,
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
        assert_eq!(sample.normalized_distance, 0.0);
    }

    #[test]
    fn horizontal_sampling_reports_distance_relative_to_feature_radius() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::ZERO);
        let to = graph.add_node(Vec3::new(10.0, 0.0, 0.0));
        graph.add_edge(from, to, 4.0, 4.0);

        let sample = graph.sample_horizontal(Vec2::new(5.0, 3.0)).unwrap();

        assert!((sample.normalized_distance - 0.75).abs() <= f32::EPSILON);
    }

    #[test]
    fn horizontal_margin_detects_nearby_feature_without_widening_base_sample() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::ZERO);
        let to = graph.add_node(Vec3::new(10.0, 0.0, 0.0));
        graph.add_edge(from, to, 2.0, 2.0);

        let nearby = Vec2::new(5.0, 5.0);
        assert!(graph.sample_horizontal(nearby).is_none());
        let expanded = graph.sample_horizontal_with_margin(nearby, 4.0).unwrap();
        assert!(expanded.normalized_distance > 1.0);
    }

    #[test]
    fn scaled_bank_sampling_reaches_the_full_width_of_wide_edges() {
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::ZERO);
        let to = graph.add_node(Vec3::new(10.0, 0.0, 0.0));
        graph.add_edge(from, to, 20.0, 20.0);
        let bank = Vec2::new(5.0, 45.0);

        assert!(graph.sample_horizontal_with_margin(bank, 16.5).is_none());
        let sampled = graph
            .sample_horizontal_with_radius_multiplier(bank, 2.5)
            .unwrap();
        assert!((sampled.normalized_distance - 2.25).abs() <= f32::EPSILON);
        assert!(
            graph
                .sample_horizontal_with_radius_multiplier(Vec2::new(5.0, 51.0), 2.5)
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
