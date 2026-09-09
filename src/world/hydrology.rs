use bevy::prelude::*;

use super::feature_graph::FeatureGraph;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterBodyKind {
    Lake,
    Ocean,
}

#[derive(Clone, Copy, Debug)]
pub struct WaterBody {
    pub kind: WaterBodyKind,
    pub center: Vec2,
    pub radius: Vec2,
    pub water_level: f32,
    pub carve_depth: f32,
}

impl WaterBody {
    pub fn horizontal_strength(&self, position: Vec2) -> f32 {
        let delta = position - self.center;
        let normalized = Vec2::new(delta.x / self.radius.x, delta.y / self.radius.y);
        let distance = normalized.length();

        1.0 - distance.clamp(0.0, 1.0)
    }

    pub fn contains_horizontal(&self, position: Vec2) -> bool {
        self.horizontal_strength(position) > 0.0
    }
}

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub coord: IVec2,
    pub river_graph: FeatureGraph,
    pub river_carve_strength: f32,
    pub water_bodies: Vec<WaterBody>,
}

impl HydrologyRegion {
    fn empty(coord: IVec2) -> Self {
        Self {
            coord,
            river_graph: FeatureGraph::default(),
            river_carve_strength: 0.0,
            water_bodies: Vec::new(),
        }
    }

    pub fn density_delta(&self, position: Vec3) -> f32 {
        let river_delta = self
            .river_graph
            .sample(position)
            .map_or(0.0, |sample| -self.river_carve_strength * sample.strength);
        let water_body_delta = self
            .water_bodies
            .iter()
            .map(|body| {
                if body.carve_depth <= 0.0 {
                    return 0.0;
                }

                let horizontal = body.horizontal_strength(Vec2::new(position.x, position.z));
                let bottom = body.water_level - body.carve_depth;

                if horizontal <= 0.0 || position.y < bottom || position.y > body.water_level {
                    return 0.0;
                }

                -body.carve_depth * horizontal
            })
            .sum::<f32>();

        river_delta + water_body_delta
    }

    pub fn water_level_at(&self, position: Vec2) -> Option<f32> {
        self.water_bodies
            .iter()
            .filter(|body| body.contains_horizontal(position))
            .map(|body| body.water_level)
            .max_by(f32::total_cmp)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologyField {
    seed: u64,
    sea_level: i32,
}

impl HydrologyField {
    pub fn new(seed: u64, sea_level: i32) -> Self {
        Self { seed, sea_level }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub fn region(&self, coord: IVec2) -> HydrologyRegion {
        HydrologyRegion::empty(coord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_body_strength_fades_to_zero_at_shoreline() {
        let body = WaterBody {
            kind: WaterBodyKind::Lake,
            center: Vec2::ZERO,
            radius: Vec2::splat(10.0),
            water_level: 64.0,
            carve_depth: 8.0,
        };

        assert_eq!(body.horizontal_strength(Vec2::ZERO), 1.0);
        assert_eq!(body.horizontal_strength(Vec2::new(10.0, 0.0)), 0.0);
    }
}
