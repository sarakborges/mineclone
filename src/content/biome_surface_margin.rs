use serde::Deserialize;

use super::biome_material::BiomeMaterialLayer;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeSurfaceMargin {
    pub width: f32,
    pub surface_layers: Vec<BiomeMaterialLayer>,
}

impl BiomeSurfaceMargin {
    pub fn block_at_depth(&self, depth: u32) -> Option<&str> {
        let mut remaining = depth;

        for layer in &self.surface_layers {
            match layer.depth {
                Some(layer_depth) if remaining >= layer_depth => remaining -= layer_depth,
                _ => return Some(layer.block.as_str()),
            }
        }

        None
    }

    pub fn validate(&self, biome_id: &str) {
        assert!(
            self.width.is_finite() && self.width > 0.0,
            "biome {biome_id} surfaceMargin.width must be positive and finite"
        );
        assert!(
            !self.surface_layers.is_empty(),
            "biome {biome_id} surfaceMargin must define surfaceLayers"
        );

        for (index, layer) in self.surface_layers.iter().enumerate() {
            assert!(
                !layer.block.trim().is_empty(),
                "biome {biome_id} surfaceMargin.surfaceLayers[{index}].block cannot be empty"
            );

            let is_last = index + 1 == self.surface_layers.len();
            if is_last {
                assert!(
                    layer.depth.is_none(),
                    "biome {biome_id} surfaceMargin final surface layer must omit depth"
                );
            } else {
                assert!(
                    matches!(layer.depth, Some(depth) if depth > 0),
                    "biome {biome_id} surfaceMargin.surfaceLayers[{index}].depth must be positive"
                );
            }
        }
    }
}
