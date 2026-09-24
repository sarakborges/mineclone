use crate::content::{object::ObjectPlacementFace, object_id::intern_object_id};

use super::texture_rotation::TextureRotation;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ObjectCell {
    pub(crate) object_id: &'static str,
    pub(crate) face: ObjectPlacementFace,
    pub(crate) rotation: TextureRotation,
}

impl ObjectCell {
    pub(crate) fn new(
        object_id: &str,
        face: ObjectPlacementFace,
        rotation: TextureRotation,
    ) -> Self {
        Self {
            object_id: intern_object_id(object_id),
            face,
            rotation,
        }
    }
}
