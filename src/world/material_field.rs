use bevy::prelude::*;

use crate::content::{
    biome::{BiomeDefinition, BiomeRegistry},
    biome_material::BiomeMaterialLayer,
    block_id::intern_block_id,
};

use super::{
    biome_field::{BiomeField, VolumeBiomeSelection},
    deterministic::{hash_string, mix_seed},
    noise::fractal_noise_2d,
};

const BIOME_MATERIAL_BLEND_SCALE: f32 = 0.07;
const BIOME_MATERIAL_BLEND_SALT: u64 = 0x6a09_e667_f3bc_c909;

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

    surface_material_id(surface, surface_materials, biome_field.seed())
        .map(intern_block_id)
        .unwrap_or_else(|| {
            panic!(
                "surface biome sample did not resolve a material at depth {}",
                surface.depth
            )
        })
}

pub(crate) fn surface_material_id<'a>(
    surface: SurfaceMaterialSample,
    surface_materials: &SurfaceMaterialColumn<'a>,
    seed: u64,
) -> Option<&'a str> {
    // Surface layers are horizontal cover. On a cliff/steep mountain edge,
    // expose the biome's deepest authored substrate instead of wrapping grass,
    // dirt, sand, fluid, or another shallow surface material down the wall.
    let material_depth = if surface.steep {
        u32::MAX
    } else {
        surface.depth
    };
    let base_material = blended_surface_material(
        surface_materials,
        material_depth,
        surface.position,
        seed,
    );
    if !surface.steep && surface.depth > 0 {
        blended_surface_material(
            surface_materials,
            irregular_layer_depth(surface.position, surface.depth, seed),
            surface.position,
            seed,
        )
        .or(base_material)
    } else {
        base_material
    }
}

fn blended_surface_material<'a>(
    surface_materials: &SurfaceMaterialColumn<'a>,
    depth: u32,
    position: Vec3,
    seed: u64,
) -> Option<&'a str> {
    if let Some(block_id) = surface_materials
        .margin
        .and_then(|biome| biome.surface_margin.as_ref())
        .and_then(|margin| margin.block_at_depth(depth))
    {
        return Some(block_id);
    }

    let mut selected = None::<(&BiomeDefinition, &BiomeMaterialLayer, f32)>;
    for influence in &surface_materials.influences {
        if influence.weight <= f32::EPSILON {
            continue;
        }
        let Some(layer) = influence.biome.surface_layer_at_depth(depth) else {
            continue;
        };
        let score = surface_material_transition_score(
            position.xz(),
            seed,
            influence.biome.id.as_str(),
            influence.weight,
        );
        if selected.is_none_or(|(selected_biome, _, selected_score)| {
            score < selected_score
                || (score == selected_score
                    && influence.biome.id.as_str() < selected_biome.id.as_str())
        }) {
            selected = Some((influence.biome, layer, score));
        }
    }

    selected.map(|(biome, layer, _)| {
        resolve_material_layer_block(biome.id.as_str(), layer, position.xz(), seed)
    })
}

fn surface_material_transition_score(
    position: Vec2,
    seed: u64,
    biome_id: &str,
    weight: f32,
) -> f32 {
    let noise_seed = mix_seed(seed ^ BIOME_MATERIAL_BLEND_SALT ^ hash_string(biome_id));
    let unit = ((fractal_noise_2d(position * BIOME_MATERIAL_BLEND_SCALE, noise_seed, 3) + 1.0)
        * 0.5)
        .clamp(f32::MIN_POSITIVE, 1.0);
    -unit.ln() / weight.max(f32::MIN_POSITIVE)
}

pub(crate) fn resolve_material_layer_block<'a>(
    biome_id: &str,
    layer: &'a BiomeMaterialLayer,
    position: Vec2,
    seed: u64,
) -> &'a str {
    if layer.alternates.is_empty() {
        return layer.block.as_str();
    }

    let variant_count = layer.alternates.len() + 1;
    let noise_seed = mix_seed(seed ^ hash_string(biome_id) ^ hash_string(&layer.block));
    let normalized =
        ((fractal_noise_2d(position * layer.patch_scale, noise_seed, 3) + 1.0) * 0.5)
            .clamp(0.0, 1.0);
    let index = ((normalized * variant_count as f32).floor() as usize).min(variant_count - 1);

    if index == 0 {
        layer.block.as_str()
    } else {
        layer.alternates[index - 1].as_str()
    }
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
    let varied_depth = (surface_depth as f32 + offset).round().max(0.0) as u32;

    if surface_depth == 0 {
        0
    } else {
        // The authored top layer is top-only. Irregular subsurface boundaries
        // may move between deeper layers, but must never pull grass, fluid, or
        // any other depth-zero material down into the terrain column.
        varied_depth.max(1)
    }
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
    fn irregular_layer_depth_never_promotes_subsurface_into_top_layer() {
        let stays_below_surface = (-64..=64).all(|index| {
            irregular_layer_depth(Vec3::new(index as f32, 60.0, index as f32 * 0.37), 1, 42) >= 1
        });

        assert!(stays_below_surface);
    }

    #[test]
    fn material_transition_score_respects_influence_weight() {
        let position = Vec2::new(37.0, -19.0);
        let low = surface_material_transition_score(position, 42, "test:plains", 0.25);
        let high = surface_material_transition_score(position, 42, "test:plains", 0.75);

        assert!(high < low);
    }

    #[test]
    fn patchy_material_layer_uses_base_and_alternate_blocks() {
        let layer = BiomeMaterialLayer {
            block: "asteria:dirt".to_owned(),
            alternates: vec!["asteria:mud".to_owned()],
            patch_scale: 0.04,
            depth: Some(1),
        };
        let blocks = (0..128)
            .map(|index| {
                resolve_material_layer_block(
                    "asteria:overworld/swamp",
                    &layer,
                    Vec2::new(index as f32 * 4.0, index as f32 * 1.7),
                    42,
                )
            })
            .collect::<std::collections::HashSet<_>>();

        assert!(blocks.contains("asteria:dirt"));
        assert!(blocks.contains("asteria:mud"));
    }
}
