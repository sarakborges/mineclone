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
    pub solid_block: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GeologyRegion {
    pub coord: IVec3,
    pub features: Vec<GeologicalFeature>,
}

impl GeologyRegion {
    pub fn density_delta(&self, position: Vec3) -> f32 {
        self.features
            .iter()
            .filter_map(|feature| {
                feature
                    .path
                    .sample(position)
                    .map(|sample| -feature.strength * sample.strength)
            })
            .sum()
    }

    pub fn solid_block_at(&self, position: Vec3) -> Option<&str> {
        self.features
            .iter()
            .filter_map(|feature| {
                let block_id = feature.solid_block.as_deref()?;
                let sample = feature.path.sample(position)?;
                Some((block_id, sample.strength))
            })
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(block_id, _)| block_id)
    }
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
