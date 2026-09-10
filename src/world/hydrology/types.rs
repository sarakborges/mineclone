use bevy::prelude::*;

use crate::content::biome_hydrology::BiomeHydrology;

use super::math::smoothstep;

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

        smoothstep(1.0 - distance.clamp(0.0, 1.0))
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

#[derive(Clone, Copy, Debug, Default)]
pub struct HydrologyTerrainSummary {
    pub minimum_elevation: f32,
    pub maximum_elevation: f32,
    pub mean_elevation: f32,
    pub mean_continentalness: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct HydrologyMacroSample {
    pub elevation: f32,
    pub continentalness: f32,
}
