use bevy::prelude::*;

use super::feature_graph::FeatureGraph;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeologicalFeatureKind {
    Trench,
    Ravine,
    Rift,
}

#[derive(Clone, Debug)]
pub struct GeologicalFeature {
    pub kind: GeologicalFeatureKind,
    pub path: FeatureGraph,
    pub strength: f32,
}

#[derive(Clone, Debug)]
pub struct GeologyRegion {
    pub coord: IVec3,
    pub features: Vec<GeologicalFeature>,
}

#[derive(Clone, Copy, Debug)]
pub struct GeologyField {
    seed: u64,
}

impl GeologyField {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn region(&self, coord: IVec3) -> GeologyRegion {
        GeologyRegion {
            coord,
            features: Vec::new(),
        }
    }
}
