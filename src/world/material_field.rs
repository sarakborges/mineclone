use bevy::prelude::*;

use crate::content::{
    biome::{BiomeDefinition, BiomeRegistry},
    block_id::intern_block_id,
};

use super::biome_field::{BiomeField, VolumeBiomeSelection};

#[derive(Clone, Copy)]
struct ResolvedSurfaceInfluence<'a> {
    biome: &'a BiomeDefinition,
    weight: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct SurfaceMaterialSample {
    pub(crate) position: Vec3,
    pub(crate) depth: u32,
    pub(crate) steep: bool,
}

#[derive(Default)]
pub(crate) struct SurfaceMaterialColumn<'a> {
    influences: Vec<ResolvedSurfaceInfluence<'a>>,
    margin: Option<&'a BiomeDefinition>,
}

pub(crate) fn resolve_surface_material_column<'a>(
    surface_influences: &[(usize, f32)],
    surface_margin_index: Option<usize>,
    biome_field: &BiomeField,
    biomes: &'a BiomeRegistry,
    column: &mut SurfaceMaterialColumn<'a>,
) {
    column.influences.clear();
    column.margin = surface_margin_index.map(|biome_index| {
        let biome_id = biome_field.surface_biome_id(biome_index);
        let biome = biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing surface margin biome definition: {biome_id}"));
        assert!(
            biome.surface_margin.is_some(),
            "resolved surface margin biome does not define surfaceMargin: {biome_id}"
        );
        biome
    });
    column
        .influences
        .extend(surface_influences.iter().map(|(biome_index, weight)| {
            let biome_id = biome_field.surface_biome_id(*biome_index);
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

            ResolvedSurfaceInfluence {
                biome,
                weight: *weight,
            }
        }));
}

pub(crate) fn solid_block_id(
    surface: SurfaceMaterialSample,
    volume: Option<VolumeBiomeSelection>,
    volume_surface_depth: Option<u32>,
    surface_materials: &SurfaceMaterialColumn<'_>,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
) -> &'static str {
    if let Some(selection) = volume {
        let biome_id = biome_field.volume_biome_id(selection);
        let volume_biome = biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing volume biome definition: {biome_id}"));

        if let Some(depth) = volume_surface_depth
            && !volume_biome.surface_layers.is_empty()
            && let Some(block_id) = volume_biome.surface_block_at_depth(depth)
        {
            return intern_block_id(block_id);
        }

        if let Some(block_id) = biome_field.volume_solid_block(selection) {
            return intern_block_id(block_id);
        }
    }

    // Surface layers are horizontal cover. On a cliff/steep mountain edge,
    // expose the biome's deepest authored substrate instead of wrapping grass,
    // dirt, sand, or another shallow surface layer down the wall.
    let material_depth = if surface.steep {
        u32::MAX
    } else {
        surface.depth
    };
    let base_material = strongest_surface_material(surface_materials, material_depth);
    let resolved_material = if !surface.steep && surface.depth > 0 {
        strongest_surface_material(
            surface_materials,
            irregular_layer_depth(surface.position, surface.depth, biome_field.seed()),
        )
        .or(base_material)
    } else {
        base_material
    };

    resolved_material.map(intern_block_id).unwrap_or_else(|| {
        panic!(
            "surface biome sample did not resolve a material at depth {}",
            surface.depth
        )
    })
}

fn strongest_surface_material<'a>(
    surface_materials: &SurfaceMaterialColumn<'a>,
    depth: u32,
) -> Option<&'a str> {
    if let Some(block_id) = surface_materials
        .margin
        .and_then(|biome| biome.surface_margin.as_ref())
        .and_then(|margin| margin.block_at_depth(depth))
    {
        return Some(block_id);
    }

    surface_materials
        .influences
        .iter()
        .filter_map(|influence| {
            influence
                .biome
                .surface_block_at_depth(depth)
                .map(|block_id| (block_id, influence.weight))
        })
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(block_id, _)| block_id)
}

fn irregular_layer_depth(position: Vec3, surface_depth: u32, seed: u64) -> u32 {
    let phase = (seed % 10_000) as f32 * 0.001;
    let broad = ((position.x * 0.055 + phase).sin()
        + (position.z * 0.071 - phase * 0.7).cos()
        + ((position.x + position.z) * 0.037 + phase * 1.3).sin())
        * 0.62;
    let detail = ((position.x * 0.19 + position.y * 0.23 - position.z * 0.17 + phase).sin()
        + (position.x * 0.13 - position.y * 0.29 + position.z * 0.21 - phase * 1.7).cos())
        * 0.38;
    let offset = broad + detail;

    (surface_depth as f32 + offset).round().max(0.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irregular_layer_depth_can_vary_across_any_boundary() {
        let depths = (0..32)
            .map(|index| {
                irregular_layer_depth(
                    Vec3::new(index as f32 * 2.0, 58.0, index as f32 * 0.75),
                    3,
                    42,
                )
            })
            .collect::<std::collections::HashSet<_>>();

        assert!(depths.len() > 1);
    }

    #[test]
    fn irregular_layer_depth_can_reach_the_surface_layer_below_depth_zero() {
        let reaches_surface = (-64..=64).any(|index| {
            irregular_layer_depth(Vec3::new(index as f32, 60.0, index as f32 * 0.37), 1, 42) == 0
        });

        assert!(reaches_surface);
    }
}
