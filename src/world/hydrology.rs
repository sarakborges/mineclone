use bevy::prelude::*;

use super::feature_graph::FeatureGraph;

const HYDROLOGY_REGION_SIZE: f32 = 128.0;
const MACRO_SAMPLE_GRID: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterBodyKind {
    Lake,
    Ocean,
}

#[derive(Clone, Debug)]
pub struct WaterBody {
    pub kind: WaterBodyKind,
    pub center: Vec2,
    pub radius: Vec2,
    pub water_level: f32,
    pub carve_depth: f32,
    pub fluid_id: String,
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

#[derive(Clone, Copy, Debug)]
pub struct HydrologyWaterSample<'a> {
    pub fluid_id: &'a str,
    pub water_level: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HydrologyTerrainSummary {
    pub minimum_elevation: f32,
    pub maximum_elevation: f32,
    pub mean_elevation: f32,
    pub mean_continentalness: f32,
}

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub coord: IVec2,
    pub terrain: HydrologyTerrainSummary,
    pub river_graph: FeatureGraph,
    pub river_carve_strength: f32,
    pub water_bodies: Vec<WaterBody>,
}

impl HydrologyRegion {
    fn empty(coord: IVec2) -> Self {
        Self {
            coord,
            terrain: HydrologyTerrainSummary::default(),
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

    pub fn water_at(&self, position: Vec2) -> Option<HydrologyWaterSample<'_>> {
        self.water_bodies
            .iter()
            .filter(|body| body.contains_horizontal(position))
            .max_by(|left, right| left.water_level.total_cmp(&right.water_level))
            .map(|body| HydrologyWaterSample {
                fluid_id: body.fluid_id.as_str(),
                water_level: body.water_level,
            })
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

    pub fn region_from_macro_terrain(
        &self,
        coord: IVec2,
        mut sample: impl FnMut(Vec2) -> (f32, f32),
    ) -> HydrologyRegion {
        let mut minimum_elevation = f32::MAX;
        let mut maximum_elevation = f32::MIN;
        let mut elevation_sum = 0.0;
        let mut continentalness_sum = 0.0;
        let mut count = 0.0;

        for z in 0..MACRO_SAMPLE_GRID {
            for x in 0..MACRO_SAMPLE_GRID {
                let position = macro_sample_position(coord, x, z);
                let (elevation, continentalness) = sample(position);

                minimum_elevation = minimum_elevation.min(elevation);
                maximum_elevation = maximum_elevation.max(elevation);
                elevation_sum += elevation;
                continentalness_sum += continentalness;
                count += 1.0;
            }
        }

        HydrologyRegion {
            coord,
            terrain: HydrologyTerrainSummary {
                minimum_elevation,
                maximum_elevation,
                mean_elevation: elevation_sum / count,
                mean_continentalness: continentalness_sum / count,
            },
            river_graph: FeatureGraph::default(),
            river_carve_strength: 0.0,
            water_bodies: Vec::new(),
        }
    }
}

fn macro_sample_position(coord: IVec2, x: usize, z: usize) -> Vec2 {
    let origin = coord.as_vec2() * HYDROLOGY_REGION_SIZE;
    let step = HYDROLOGY_REGION_SIZE / (MACRO_SAMPLE_GRID - 1) as f32;

    origin + Vec2::new(x as f32 * step, z as f32 * step)
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
            fluid_id: "mineclone:water".into(),
        };

        assert_eq!(body.horizontal_strength(Vec2::ZERO), 1.0);
        assert_eq!(body.horizontal_strength(Vec2::new(10.0, 0.0)), 0.0);
    }

    #[test]
    fn macro_terrain_summary_is_deterministic() {
        let field = HydrologyField::new(42, 64);
        let sample = |position: Vec2| (position.x + position.y, 0.5);
        let first = field.region_from_macro_terrain(IVec2::ZERO, sample);
        let second = field.region_from_macro_terrain(IVec2::ZERO, sample);

        assert_eq!(first.terrain.minimum_elevation, second.terrain.minimum_elevation);
        assert_eq!(first.terrain.maximum_elevation, second.terrain.maximum_elevation);
        assert_eq!(first.terrain.mean_elevation, second.terrain.mean_elevation);
        assert_eq!(first.terrain.mean_continentalness, 0.5);
    }

    #[test]
    fn adjacent_regions_sample_the_same_shared_boundary() {
        let left_boundary = macro_sample_position(IVec2::ZERO, MACRO_SAMPLE_GRID - 1, 2);
        let right_boundary = macro_sample_position(IVec2::X, 0, 2);

        assert_eq!(left_boundary, right_boundary);
    }
}
