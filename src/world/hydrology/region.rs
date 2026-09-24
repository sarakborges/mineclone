mod density;
mod material;
pub(crate) use material::HydrologyMaterialSet;
mod water;

use bevy::prelude::*;

use crate::{content::dimension_hydrology::DimensionHydrology, world::feature_graph::FeatureGraph};

use super::types::WaterBody;

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    // GenerationRegion and its hydrology must identify the same region at
    // chunk seams; allow the generation layer to check this invariant.
    pub(crate) coord: IVec2,
    pub(super) river_graph: FeatureGraph,
    pub(super) river_carve_depth: f32,
    pub(super) water_bodies: Vec<WaterBody>,
    pub(super) settings: DimensionHydrology,
}
