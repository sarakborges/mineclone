mod density;
mod material;
mod sampling;
mod water;

use bevy::prelude::*;

use crate::{content::dimension_hydrology::DimensionHydrology, world::feature_graph::FeatureGraph};

use super::{
    constants::RIVER_CARVE_DEPTH,
    types::{HydrologyMacroSample, WaterBody},
};

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub(super) coord: IVec2,
    pub(super) river_graph: FeatureGraph,
    pub(super) river_carve_depth: f32,
    pub(super) water_bodies: Vec<WaterBody>,
    pub(super) sea_level: f32,
    pub(super) settings: DimensionHydrology,
    pub(super) macro_samples: Vec<HydrologyMacroSample>,
}

impl HydrologyRegion {
    pub(super) fn empty(coord: IVec2, sea_level: f32, settings: DimensionHydrology) -> Self {
        Self {
            coord,
            river_graph: FeatureGraph::default(),
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies: Vec::new(),
            sea_level,
            settings,
            macro_samples: Vec::new(),
        }
    }
}
