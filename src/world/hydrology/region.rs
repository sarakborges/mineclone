mod density;
mod material;
mod sampling;
mod water;

use bevy::prelude::*;

use crate::{content::dimension_hydrology::DimensionHydrology, world::feature_graph::FeatureGraph};

use super::types::{HydrologyMacroSample, WaterBody};

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub(super) coord: IVec2,
    pub(super) river_graph: FeatureGraph,
    pub(super) river_carve_depth: f32,
    pub(super) water_bodies: Vec<WaterBody>,
    pub(super) sea_level: f32,
    pub(super) settings: DimensionHydrology,
    pub(super) ocean_weight: f32,
    pub(super) macro_samples: Vec<HydrologyMacroSample>,
}
