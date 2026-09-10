use bevy::prelude::*;

use crate::content::biome_hydrology::BiomeHydrology;

use super::math::{hash_unit, smoothstep};

#[derive(Clone, Debug)]
pub struct WaterBody {
    pub center: Vec2,
    pub radius: Vec2,
    pub rotation: f32,
    pub shape_seed: u64,
    pub water_level: f32,
    pub carve_depth: f32,
    pub fluid_id: String,
}

impl WaterBody {
    pub fn normalized_horizontal_distance(&self, position: Vec2) -> f32 {
        let delta = position - self.center;
        let (sin, cos) = self.rotation.sin_cos();
        let local = Vec2::new(
            delta.x * cos + delta.y * sin,
            -delta.x * sin + delta.y * cos,
        );
        let normalized = Vec2::new(local.x / self.radius.x, local.y / self.radius.y);
        let boundary_scale = irregular_boundary_scale(normalized, self.shape_seed);

        normalized.length() / boundary_scale
    }

    pub fn horizontal_strength(&self, position: Vec2) -> f32 {
        let distance = self.normalized_horizontal_distance(position);

        smoothstep(1.0 - distance.clamp(0.0, 1.0))
    }

    pub fn contains_horizontal(&self, position: Vec2) -> bool {
        self.horizontal_strength(position) > 0.0
    }

    pub fn maximum_horizontal_extent(&self) -> f32 {
        self.radius.max_element() * 1.3
    }
}

fn irregular_boundary_scale(normalized: Vec2, seed: u64) -> f32 {
    let angle = normalized.y.atan2(normalized.x);
    let phase_a = hash_unit(seed) * std::f32::consts::TAU;
    let phase_b = hash_unit(seed.rotate_left(21)) * std::f32::consts::TAU;
    let phase_c = hash_unit(seed.rotate_left(43)) * std::f32::consts::TAU;
    let broad = (angle * 2.0 + phase_a).sin() * 0.14;
    let medium = (angle * 3.0 + phase_b).sin() * 0.09;
    let detail = (angle * 5.0 + phase_c).sin() * 0.05;

    (1.0 + broad + medium + detail).clamp(0.72, 1.28)
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologyWaterSample<'a> {
    pub fluid_id: &'a str,
    pub water_level: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologySurfaceSample {
    pub elevation: f32,
    pub continentalness: f32,
    pub biome_hydrology: BiomeHydrology,
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologyBiomeOverlay<'a> {
    pub surface_weight: f32,
    pub coast_biome: Option<&'a str>,
    pub coast_weight: f32,
    pub ocean_biome: Option<&'a str>,
    pub ocean_weight: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct HydrologyMacroSample {
    pub elevation: f32,
    pub continentalness: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lake_boundary_is_not_a_perfect_ellipse() {
        let body = WaterBody {
            center: Vec2::ZERO,
            radius: Vec2::splat(20.0),
            rotation: 0.0,
            shape_seed: 42,
            water_level: 64.0,
            carve_depth: 10.0,
            fluid_id: "asteria:test/water".into(),
        };
        let x_distance = body.normalized_horizontal_distance(Vec2::new(20.0, 0.0));
        let diagonal = Vec2::splat(20.0 / std::f32::consts::SQRT_2);
        let diagonal_distance = body.normalized_horizontal_distance(diagonal);

        assert_ne!(x_distance, diagonal_distance);
    }
}
