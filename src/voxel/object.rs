use bevy::prelude::Vec3;

use crate::content::{object::ObjectPlacementFace, object_id::intern_object_id};

use super::texture_rotation::TextureRotation;

const OBJECT_TRANSFORM_UNITS: f32 = 1024.0;
const DEFAULT_SCALE_UNITS: u16 = OBJECT_TRANSFORM_UNITS as u16;
const MAX_LOCAL_OFFSET: f32 = 8.0;
const MAX_LOCAL_SCALE: f32 = 8.0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ObjectTransform {
    offset: [i16; 3],
    scale: [u16; 3],
}

impl Default for ObjectTransform {
    fn default() -> Self {
        Self {
            offset: [0; 3],
            scale: [DEFAULT_SCALE_UNITS; 3],
        }
    }
}

impl ObjectTransform {
    pub(crate) fn from_parts(offset: Vec3, scale: Vec3) -> Option<Self> {
        if !offset.is_finite()
            || !scale.is_finite()
            || offset.abs().max_element() > MAX_LOCAL_OFFSET
            || scale.min_element() <= 0.0
            || scale.max_element() > MAX_LOCAL_SCALE
        {
            return None;
        }

        Some(Self {
            offset: [
                quantize_signed(offset.x)?,
                quantize_signed(offset.y)?,
                quantize_signed(offset.z)?,
            ],
            scale: [
                quantize_unsigned(scale.x)?,
                quantize_unsigned(scale.y)?,
                quantize_unsigned(scale.z)?,
            ],
        })
    }

    pub(crate) fn from_encoded(offset: [i16; 3], scale: [u16; 3]) -> Option<Self> {
        if scale.contains(&0) {
            return None;
        }
        let transform = Self { offset, scale };
        (transform.offset().abs().max_element() <= MAX_LOCAL_OFFSET
            && transform.scale().max_element() <= MAX_LOCAL_SCALE)
            .then_some(transform)
    }

    pub(crate) fn offset(self) -> Vec3 {
        Vec3::new(
            self.offset[0] as f32,
            self.offset[1] as f32,
            self.offset[2] as f32,
        ) / OBJECT_TRANSFORM_UNITS
    }

    pub(crate) fn scale(self) -> Vec3 {
        Vec3::new(
            self.scale[0] as f32,
            self.scale[1] as f32,
            self.scale[2] as f32,
        ) / OBJECT_TRANSFORM_UNITS
    }

    pub(crate) fn encoded_offset(self) -> [i16; 3] {
        self.offset
    }

    pub(crate) fn encoded_scale(self) -> [u16; 3] {
        self.scale
    }

    pub(crate) fn is_identity(self) -> bool {
        self == Self::default()
    }
}

fn quantize_signed(value: f32) -> Option<i16> {
    let units = (value * OBJECT_TRANSFORM_UNITS).round();
    (units >= i16::MIN as f32 && units <= i16::MAX as f32).then_some(units as i16)
}

fn quantize_unsigned(value: f32) -> Option<u16> {
    let units = (value * OBJECT_TRANSFORM_UNITS).round();
    (units >= 1.0 && units <= u16::MAX as f32).then_some(units as u16)
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ObjectCell {
    pub(crate) object_id: &'static str,
    pub(crate) face: ObjectPlacementFace,
    pub(crate) rotation: TextureRotation,
    pub(crate) transform: ObjectTransform,
}

impl ObjectCell {
    pub(crate) fn new(
        object_id: &str,
        face: ObjectPlacementFace,
        rotation: TextureRotation,
    ) -> Self {
        Self::with_transform(object_id, face, rotation, ObjectTransform::default())
    }

    pub(crate) fn with_transform(
        object_id: &str,
        face: ObjectPlacementFace,
        rotation: TextureRotation,
        transform: ObjectTransform,
    ) -> Self {
        Self {
            object_id: intern_object_id(object_id),
            face,
            rotation,
            transform,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectTransform;
    use bevy::prelude::Vec3;

    #[test]
    fn object_transform_round_trips_quantized_values() {
        let transform = ObjectTransform::from_parts(
            Vec3::new(0.125, -0.25, 0.375),
            Vec3::new(1.0, 0.5, 2.0),
        )
        .expect("valid attached object transform");

        assert_eq!(transform.offset(), Vec3::new(0.125, -0.25, 0.375));
        assert_eq!(transform.scale(), Vec3::new(1.0, 0.5, 2.0));
        assert!(!transform.is_identity());
    }

    #[test]
    fn object_transform_rejects_invalid_scale() {
        assert!(ObjectTransform::from_parts(Vec3::ZERO, Vec3::ZERO).is_none());
    }
}
