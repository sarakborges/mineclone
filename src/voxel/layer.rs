use crate::content::{layer::LayerFace, layer_id::intern_layer_id};

use super::{block_face::BlockFace, texture_rotation::TextureRotation};

pub(crate) const MAX_LAYERS_PER_VOXEL: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayerCell {
    pub layer_id: &'static str,
    pub texture_rotation: TextureRotation,
}

impl LayerCell {
    pub fn new(layer_id: &str, texture_rotation: TextureRotation) -> Self {
        Self {
            layer_id: intern_layer_id(layer_id),
            texture_rotation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AttachedLayer {
    pub(crate) face: LayerFace,
    pub(crate) cell: LayerCell,
}

pub(crate) fn block_face(face: LayerFace) -> BlockFace {
    match face {
        LayerFace::Right => BlockFace::Right,
        LayerFace::Left => BlockFace::Left,
        LayerFace::Top => BlockFace::Top,
        LayerFace::Bottom => BlockFace::Bottom,
        LayerFace::Front => BlockFace::Front,
        LayerFace::Back => BlockFace::Back,
    }
}
