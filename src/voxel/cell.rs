use crate::content::block_orientation::BlockOrientation;

use super::{
    secondary_properties::SecondaryProperties,
    texture_rotation::TextureRotation,
};

#[derive(Clone, Copy)]
pub struct VoxelCell {
    pub block_id: &'static str,
    pub texture_rotation: TextureRotation,
    pub orientation: BlockOrientation,
    secondary_properties: SecondaryProperties,
}

impl VoxelCell {
    pub fn new(block_id: &'static str, texture_rotation: TextureRotation) -> Self {
        Self::oriented(block_id, texture_rotation, BlockOrientation::default())
    }

    pub fn oriented(
        block_id: &'static str,
        texture_rotation: TextureRotation,
        orientation: BlockOrientation,
    ) -> Self {
        Self {
            block_id,
            texture_rotation,
            orientation,
            secondary_properties: SecondaryProperties::default(),
        }
    }

    pub fn with_secondary_property(mut self, property: &str, value: &str) -> Self {
        self.secondary_properties.set(property, value);
        self
    }

    pub fn secondary_property(self, property: &str) -> Option<&'static str> {
        self.secondary_properties.get(property)
    }

    pub(crate) fn secondary_properties(self) -> SecondaryProperties {
        self.secondary_properties
    }

    pub(crate) fn with_secondary_properties(mut self, properties: SecondaryProperties) -> Self {
        self.secondary_properties = properties;
        self
    }
}
