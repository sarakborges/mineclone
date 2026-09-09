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
}

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub coord: IVec2,
    pub river_graph: FeatureGraph,
    pub water_bodies: Vec<WaterBody>,
}

impl HydrologyRegion {
    fn empty(coord: IVec2) -> Self {
        Self {
            coord,
            river_graph: FeatureGraph::default(),
            water_bodies: Vec::new(),
        }
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
