use bevy::prelude::*;

use crate::world::math::{lerp, smoothstep};

use super::hash::{hash_unit, position_hash};

const UNDERGROUND_LAKE_CHANCE: f32 = 0.38;
const UNDERGROUND_LAKE_MINIMUM_RADIUS: f32 = 8.0;
const UNDERGROUND_LAKE_MAXIMUM_RADIUS: f32 = 22.0;
const UNDERGROUND_LAKE_MINIMUM_DEPTH: f32 = 3.0;
const UNDERGROUND_LAKE_MAXIMUM_DEPTH: f32 = 7.5;
const UNDERGROUND_LAKE_SURFACE_OFFSET: f32 = 1.25;
const MINIMUM_WATER_Y: f32 = 2.0;

#[derive(Clone, Debug)]
pub(super) struct UndergroundLake {
    center: Vec2,
    radius: Vec2,
    rotation: f32,
    shape_seed: u64,
    pub(super) water_level: f32,
    pub(super) depth: f32,
}

impl UndergroundLake {
    pub(super) fn horizontal_strength(&self, position: Vec2) -> f32 {
        let delta = position - self.center;
        let (sin, cos) = self.rotation.sin_cos();
        let local = Vec2::new(
            delta.x * cos + delta.y * sin,
            -delta.x * sin + delta.y * cos,
        );
        let normalized = Vec2::new(local.x / self.radius.x, local.y / self.radius.y);
        let boundary_scale = irregular_boundary_scale(normalized, self.shape_seed);
        let distance = normalized.length() / boundary_scale;

        smoothstep(1.0 - distance.clamp(0.0, 1.0))
    }
}

pub(super) fn lake_for_anchor(anchor: Vec3, seed: u64) -> Option<UndergroundLake> {
    if anchor.y - UNDERGROUND_LAKE_SURFACE_OFFSET < MINIMUM_WATER_Y
        || !anchor_has_lake(anchor, seed)
    {
        return None;
    }

    let hash = position_hash(anchor, seed ^ 0x1f83_d9ab_fb41_bd6b);
    let base_radius = lerp(
        UNDERGROUND_LAKE_MINIMUM_RADIUS,
        UNDERGROUND_LAKE_MAXIMUM_RADIUS,
        hash_unit(hash.rotate_left(11)),
    );
    let aspect = lerp(0.72, 1.28, hash_unit(hash.rotate_left(29)));

    Some(UndergroundLake {
        center: Vec2::new(anchor.x, anchor.z),
        radius: Vec2::new(base_radius * aspect, base_radius * (2.0 - aspect)),
        rotation: hash_unit(hash.rotate_left(43)) * std::f32::consts::TAU,
        shape_seed: hash.rotate_left(7),
        water_level: anchor.y - UNDERGROUND_LAKE_SURFACE_OFFSET,
        depth: lerp(
            UNDERGROUND_LAKE_MINIMUM_DEPTH,
            UNDERGROUND_LAKE_MAXIMUM_DEPTH,
            hash_unit(hash.rotate_left(53)),
        ),
    })
}

pub(super) fn anchor_has_lake(anchor: Vec3, seed: u64) -> bool {
    if anchor.y - UNDERGROUND_LAKE_SURFACE_OFFSET < MINIMUM_WATER_Y {
        return false;
    }

    let hash = position_hash(anchor, seed ^ 0xa54f_f53a_5f1d_36f1);
    hash_unit(hash.rotate_left(19)) < UNDERGROUND_LAKE_CHANCE
}

fn irregular_boundary_scale(normalized: Vec2, seed: u64) -> f32 {
    let angle = normalized.y.atan2(normalized.x);
    let phase_a = hash_unit(seed) * std::f32::consts::TAU;
    let phase_b = hash_unit(seed.rotate_left(21)) * std::f32::consts::TAU;
    let phase_c = hash_unit(seed.rotate_left(43)) * std::f32::consts::TAU;
    let broad = (angle * 2.0 + phase_a).sin() * 0.16;
    let medium = (angle * 3.0 + phase_b).sin() * 0.10;
    let detail = (angle * 5.0 + phase_c).sin() * 0.06;

    (1.0 + broad + medium + detail).clamp(0.68, 1.32)
}
