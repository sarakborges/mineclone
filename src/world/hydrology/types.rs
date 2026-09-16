use bevy::prelude::*;

use crate::{
    content::biome_hydrology::BiomeHydrology,
    world::deterministic::{avalanche_u64, hash_signed},
};

use super::math::{hash_unit, lerp, smoothstep};

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
        self.normalized_horizontal_distance_with_margin(position, 0.0)
    }

    pub fn normalized_horizontal_distance_with_margin(&self, position: Vec2, margin: f32) -> f32 {
        let delta = position - self.center;
        let (sin, cos) = self.rotation.sin_cos();
        let local = Vec2::new(
            delta.x * cos + delta.y * sin,
            -delta.x * sin + delta.y * cos,
        );
        let expanded_radius = self.radius + Vec2::splat(margin.max(0.0));
        let normalized = Vec2::new(local.x / expanded_radius.x, local.y / expanded_radius.y);
        let boundary_scale = irregular_boundary_scale(normalized, self.shape_seed);

        normalized.length() / boundary_scale
    }

    pub fn horizontal_strength(&self, position: Vec2) -> f32 {
        self.horizontal_strength_with_margin(position, 0.0)
    }

    pub fn horizontal_strength_with_margin(&self, position: Vec2, margin: f32) -> f32 {
        let distance = self.normalized_horizontal_distance_with_margin(position, margin);

        smoothstep(1.0 - distance.clamp(0.0, 1.0))
    }

    pub fn maximum_horizontal_extent(&self) -> f32 {
        self.radius.max_element() * 1.42
    }
}

fn irregular_boundary_scale(normalized: Vec2, seed: u64) -> f32 {
    let angle = normalized.y.atan2(normalized.x);
    let broad = cyclic_boundary_noise(angle, seed ^ 0x243f_6a88_85a3_08d3, 7);
    let medium = cyclic_boundary_noise(angle, seed ^ 0x1319_8a2e_0370_7344, 13);
    let detail = cyclic_boundary_noise(angle, seed ^ 0xa409_3822_299f_31d0, 23);
    let phase = hash_unit(seed.rotate_left(37)) * std::f32::consts::TAU;
    let asymmetric_lobe = (angle + phase).sin() * 0.10;

    (1.0 + broad * 0.23 + medium * 0.13 + detail * 0.07 + asymmetric_lobe).clamp(0.58, 1.42)
}

fn cyclic_boundary_noise(angle: f32, seed: u64, segments: u32) -> f32 {
    let turn = (angle / std::f32::consts::TAU).rem_euclid(1.0);
    let scaled = turn * segments as f32;
    let first = scaled.floor() as u32 % segments;
    let second = (first + 1) % segments;
    let t = smoothstep(scaled - scaled.floor());
    let sample = |index: u32| {
        let mixed = avalanche_u64(seed ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        hash_signed(mixed)
    };

    lerp(sample(first), sample(second), t)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HydrologyWaterKind {
    Lake,
    River,
    Ocean,
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologyWaterSample<'a> {
    pub fluid_id: &'a str,
    pub water_level: f32,
    pub bed_level: f32,
    pub strength: f32,
    pub(crate) kind: HydrologyWaterKind,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HydrologyRiverSurfaceSample {
    pub water_level: f32,
    pub strength: f32,
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

    #[test]
    fn lake_boundary_has_asymmetric_large_scale_variation() {
        let body = WaterBody {
            center: Vec2::ZERO,
            radius: Vec2::splat(40.0),
            rotation: 0.0,
            shape_seed: 91,
            water_level: 64.0,
            carve_depth: 10.0,
            fluid_id: "asteria:test/water".into(),
        };

        let east = body.normalized_horizontal_distance(Vec2::new(40.0, 0.0));
        let west = body.normalized_horizontal_distance(Vec2::new(-40.0, 0.0));

        assert_ne!(east, west);
    }

    #[test]
    fn water_body_margin_detects_nearby_positions_without_expanding_base_shape() {
        let body = WaterBody {
            center: Vec2::ZERO,
            radius: Vec2::splat(10.0),
            rotation: 0.0,
            shape_seed: 42,
            water_level: 64.0,
            carve_depth: 10.0,
            fluid_id: "asteria:test/water".into(),
        };
        let nearby = Vec2::new(16.0, 0.0);

        assert_eq!(body.horizontal_strength(nearby), 0.0);
        assert!(body.horizontal_strength_with_margin(nearby, 8.0) > 0.0);
    }
}
