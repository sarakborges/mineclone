use crate::content::block_orientation::BlockOrientation;

use super::{
    secondary_properties::{BlockState, SecondaryProperties},
    texture_rotation::TextureRotation,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VoxelCell {
    pub block_id: &'static str,
    pub texture_rotation: TextureRotation,
    pub orientation: BlockOrientation,
    block_state: BlockState,
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
            block_state: BlockState::default(),
            microblock_layers: None,
            microblock_transient: false,
        }
    }

    pub(crate) fn with_block_id(mut self, block_id: &'static str) -> Self {
        self.block_id = block_id;
        self
    }

    pub fn with_state(mut self, property: &str, value: &str) -> Self {
        self.block_state.set(property, value);
        self
    }

    pub fn without_state(mut self, property: &str) -> Self {
        self.block_state.remove(property);
        self
    }

    pub fn state(self, property: &str) -> Option<&'static str> {
        self.block_state.get(property)
    }

    pub(crate) fn block_state(self) -> BlockState {
        self.block_state
    }

    pub(crate) fn with_block_state(mut self, state: BlockState) -> Self {
        self.block_state = state;
        self
    }

    /// Compatibility API while callers migrate from the old terminology.
    pub fn with_secondary_property(self, property: &str, value: &str) -> Self {
        self.with_state(property, value)
    }

    /// Compatibility API while callers migrate from the old terminology.
    pub fn without_secondary_property(self, property: &str) -> Self {
        self.without_state(property)
    }

    /// Compatibility API while callers migrate from the old terminology.
    pub fn secondary_property(self, property: &str) -> Option<&'static str> {
        self.state(property)
    }

    /// Compatibility API while save/load callers migrate from the old terminology.
    pub(crate) fn secondary_properties(self) -> SecondaryProperties {
        self.block_state
    }

    /// Compatibility API while save/load callers migrate from the old terminology.
    pub(crate) fn with_secondary_properties(self, properties: SecondaryProperties) -> Self {
        self.with_block_state(properties)
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

#[cfg(test)]
mod tests {
    use super::VoxelCell;
    use crate::voxel::texture_rotation::TextureRotation;

    #[test]
    fn canonical_block_state_api_preserves_compact_state() {
        let cell = VoxelCell::new("stone", TextureRotation::Degrees0)
            .with_state("variant", "mossy");

        assert_eq!(cell.state("variant"), Some("mossy"));
        assert_eq!(cell.secondary_property("variant"), Some("mossy"));
    }
}
