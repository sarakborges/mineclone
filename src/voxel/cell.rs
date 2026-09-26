use crate::content::block_orientation::BlockOrientation;

use super::{secondary_properties::SecondaryProperties, texture_rotation::TextureRotation};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VoxelCell {
    pub block_id: &'static str,
    pub texture_rotation: TextureRotation,
    pub orientation: BlockOrientation,
    secondary_properties: SecondaryProperties,
    microblock_layers: Option<&'static [u64; 8]>,
    microblock_transient: bool,
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
            microblock_layers: None,
            microblock_transient: false,
        }
    }

    pub(crate) fn with_block_id(mut self, block_id: &'static str) -> Self {
        self.block_id = block_id;
        self
    }

    pub fn with_secondary_property(mut self, property: &str, value: &str) -> Self {
        self.secondary_properties.set(property, value);
        self
    }

    pub fn without_secondary_property(mut self, property: &str) -> Self {
        self.secondary_properties.remove(property);
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

    pub(crate) fn microblock_layers(self) -> Option<&'static [u64; 8]> {
        self.microblock_layers
    }

    pub(crate) fn microblock_transient(self) -> bool {
        self.microblock_transient
    }

    pub(crate) fn with_microblock_mask(
        mut self,
        layers: Option<&'static [u64; 8]>,
        transient: bool,
    ) -> Self {
        self.microblock_layers = layers;
        self.microblock_transient = layers.is_some() && transient;
        self
    }
}
