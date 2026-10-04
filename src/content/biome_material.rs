use serde::Deserialize;

use super::{
    biome::{BiomeDefinition, BiomeKind},
    block::BlockRegistry,
    fluid::FluidRegistry,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeMaterialLayer {
    pub block: String,
    #[serde(default)]
    pub alternates: Vec<String>,
    #[serde(default = "default_patch_scale")]
    pub patch_scale: f32,
    #[serde(default)]
    pub depth: Option<u32>,
}

impl BiomeDefinition {
    pub fn surface_layer_at_depth(&self, depth: u32) -> Option<&BiomeMaterialLayer> {
        let mut remaining = depth;

        for layer in &self.surface_layers {
            match layer.depth {
                Some(layer_depth) if remaining >= layer_depth => remaining -= layer_depth,
                _ => return Some(layer),
            }
        }

        None
    }

    pub fn surface_block_at_depth(&self, depth: u32) -> Option<&str> {
        self.surface_layer_at_depth(depth)
            .map(|layer| layer.block.as_str())
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
            for (alternate_index, alternate) in layer.alternates.iter().enumerate() {
                assert!(
                    !alternate.trim().is_empty(),
                    "biome {} surfaceLayers[{index}].alternates[{alternate_index}] cannot be empty",
                    self.id
                );
            }
            assert!(
                layer.patch_scale.is_finite() && layer.patch_scale > 0.0,
                "biome {} surfaceLayers[{index}].patchScale must be positive and finite",
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

    pub(crate) fn validate_material_references(
        &self,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
    ) {
        for (index, layer) in self.surface_layers.iter().enumerate() {
            validate_layer_material_references(
                self,
                layer,
                index,
                "surfaceLayers",
                blocks,
                fluids,
            );
        }

        if let Some(margin) = &self.surface_margin {
            for (index, layer) in margin.surface_layers.iter().enumerate() {
                validate_layer_material_references(
                    self,
                    layer,
                    index,
                    "surfaceMargin.surfaceLayers",
                    blocks,
                    fluids,
                );
            }
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

fn validate_layer_material_references(
    biome: &BiomeDefinition,
    layer: &BiomeMaterialLayer,
    layer_index: usize,
    field: &str,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) {
    for material in std::iter::once(&layer.block).chain(layer.alternates.iter()) {
        if blocks.get(material).is_some() {
            continue;
        }

        assert!(
            fluids.id_of(material).is_some(),
            "biome {} {field}[{layer_index}] references missing block or fluid: {material}",
            biome.id
        );
        assert!(
            biome.kind == BiomeKind::Surface,
            "biome {} {field}[{layer_index}] fluid material {material} is only valid for surface biomes",
            biome.id
        );
        assert!(
            layer_index == 0 && layer.depth == Some(1),
            "biome {} {field}[{layer_index}] fluid material {material} must be in a one-voxel top surface layer",
            biome.id
        );
    }
}

fn default_patch_scale() -> f32 {
    0.04
}
