use crate::content::{
    biome::{BiomeDefinition, BiomeRegistry},
    block_id::intern_block_id,
};

use super::{
    biome_field::{BiomeField, VolumeBiomeSelection},
    geology::GeologyRegion,
    hydrology::HydrologyRegion,
};

#[derive(Clone, Copy)]
struct ResolvedSurfaceInfluence<'a> {
    biome: &'a BiomeDefinition,
    weight: f32,
}

pub(crate) struct SurfaceMaterialColumn<'a> {
    influences: Vec<ResolvedSurfaceInfluence<'a>>,
}

pub(crate) struct MaterialFieldContext<'a> {
    pub biome_field: &'a BiomeField,
    pub geology: &'a GeologyRegion,
    pub hydrology: &'a HydrologyRegion,
    pub biomes: &'a BiomeRegistry,
}

pub(crate) fn resolve_surface_material_column<'a>(
    surface_influences: &[(usize, f32)],
    biome_field: &BiomeField,
    biomes: &'a BiomeRegistry,
) -> SurfaceMaterialColumn<'a> {
    let influences = surface_influences
        .iter()
        .map(|(biome_index, weight)| {
            let biome_id = biome_field.surface_biome_id(*biome_index);
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

            ResolvedSurfaceInfluence {
                biome,
                weight: *weight,
            }
        })
        .collect();

    SurfaceMaterialColumn { influences }
}

pub(crate) fn solid_block_id(
    position: bevy::prelude::Vec3,
    surface_influences: &[(usize, f32)],
    surface_depth: u32,
    volume: Option<VolumeBiomeSelection>,
    context: &MaterialFieldContext<'_>,
) -> &'static str {
    let hydrology_block = context.hydrology.solid_block_at(position);

    solid_block_id_with_hydrology(
        position,
        surface_influences,
        surface_depth,
        volume,
        hydrology_block,
        context,
    )
}

pub(crate) fn solid_block_id_with_hydrology(
    position: bevy::prelude::Vec3,
    surface_influences: &[(usize, f32)],
    surface_depth: u32,
    volume: Option<VolumeBiomeSelection>,
    hydrology_block: Option<&str>,
    context: &MaterialFieldContext<'_>,
) -> &'static str {
    let surface_materials = resolve_surface_material_column(
        surface_influences,
        context.biome_field,
        context.biomes,
    );

    solid_block_id_with_resolved_surface(
        position,
        surface_depth,
        volume,
        hydrology_block,
        &surface_materials,
        context,
    )
}

pub(crate) fn solid_block_id_with_resolved_surface(
    position: bevy::prelude::Vec3,
    surface_depth: u32,
    volume: Option<VolumeBiomeSelection>,
    hydrology_block: Option<&str>,
    surface_materials: &SurfaceMaterialColumn<'_>,
    context: &MaterialFieldContext<'_>,
) -> &'static str {
    if let Some(block_id) =
        volume.and_then(|selection| context.biome_field.volume_solid_block(selection))
    {
        return intern_block_id(block_id);
    }

    if let Some(block_id) = context.geology.solid_block_at(position) {
        return intern_block_id(block_id);
    }

    if let Some(block_id) = hydrology_block {
        return intern_block_id(block_id);
    }

    let base_material = strongest_resolved_surface_material(surface_materials, surface_depth);
    let resolved_material = if surface_depth > 0 {
        strongest_resolved_surface_material(
            surface_materials,
            irregular_layer_depth(position, surface_depth, context.biome_field.seed()),
        )
        .or(base_material)
    } else {
        base_material
    };

    resolved_material.map(intern_block_id).unwrap_or_else(|| {
        panic!("surface biome sample did not resolve a material at depth {surface_depth}")
    })
}

fn strongest_resolved_surface_material<'a>(
    surface_materials: &SurfaceMaterialColumn<'a>,
    depth: u32,
) -> Option<&'a str> {
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

fn irregular_layer_depth(
    position: bevy::prelude::Vec3,
    surface_depth: u32,
    seed: u64,
) -> u32 {
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
                    bevy::prelude::Vec3::new(
                        index as f32 * 2.0,
                        58.0,
                        index as f32 * 0.75,
                    ),
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
            irregular_layer_depth(
                bevy::prelude::Vec3::new(index as f32, 60.0, index as f32 * 0.37),
                1,
                42,
            ) == 0
        });

        assert!(reaches_surface);
    }
}
