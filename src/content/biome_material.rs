use serde::Deserialize;

use super::{biome::BiomeDefinition, block::BlockRegistry};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeMaterialLayer {
    pub block: String,
    #[serde(default)]
    pub depth: Option<u32>,
}

impl BiomeDefinition {
    pub fn surface_block_at_depth(&self, depth: u32) -> Option<&str> {
        let mut remaining = depth;

        for layer in &self.surface_layers {
            match layer.depth {
                Some(layer_depth) if remaining >= layer_depth => remaining -= layer_depth,
                _ => return Some(layer.block.as_str()),
            }
        }

        None
    }

    pub(crate) fn validate_surface_materials(&self) {
        assert!(
            !self.surface_layers.is_empty(),
            "surface biome {} must define surfaceLayers",
            self.id
        );

        for (index, layer) in self.surface_layers.iter().enumerate() {
            assert!(
                !layer.block.trim().is_empty(),
                "biome {} surfaceLayers[{index}].block cannot be empty",
                self.id
            );

            let is_last = index + 1 == self.surface_layers.len();
            if is_last {
                assert!(
                    layer.depth.is_none(),
                    "biome {} final surface layer must omit depth so it can fill all remaining terrain",
                    self.id
                );
            } else {
                assert!(
                    matches!(layer.depth, Some(depth) if depth > 0),
                    "biome {} surfaceLayers[{index}].depth must be positive",
                    self.id
                );
            }
        }
    }

    pub(crate) fn validate_material_references(&self, blocks: &BlockRegistry) {
        for layer in &self.surface_layers {
            assert!(
                blocks.get(&layer.block).is_some(),
                "biome {} surface layer references missing block: {}",
                self.id,
                layer.block
            );
        }

        if let Some(block_id) = self.solid_block.as_deref() {
            assert!(
                blocks.get(block_id).is_some(),
                "biome {} solidBlock references missing block: {block_id}",
                self.id
            );
        }
    }
}
