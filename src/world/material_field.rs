use crate::content::{biome::BiomeRegistry, block_id::intern_block_id};

use super::{
    biome_field::{BiomeField, BiomeFieldSample, VolumeBiomeSelection},
    geology::GeologyRegion,
    hydrology::HydrologyRegion,
};

pub(crate) struct MaterialFieldContext<'a> {
    pub biome_field: &'a BiomeField,
    pub geology: &'a GeologyRegion,
    pub hydrology: &'a HydrologyRegion,
    pub biomes: &'a BiomeRegistry,
}

pub(crate) fn solid_block_id(
    position: bevy::prelude::Vec3,
    surface: &BiomeFieldSample<'_>,
    surface_depth: u32,
    volume: Option<VolumeBiomeSelection>,
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

    if let Some(block_id) = context.hydrology.solid_block_at(position) {
        return intern_block_id(block_id);
    }

    strongest_surface_material(
        surface
            .influences
            .iter()
            .map(|influence| (influence.id, influence.weight)),
        surface_depth,
        context.biomes,
    )
    .map(intern_block_id)
    .unwrap_or_else(|| {
        panic!("surface biome sample did not resolve a material at depth {surface_depth}")
    })
}

fn strongest_surface_material<'registry, 'id>(
    influences: impl Iterator<Item = (&'id str, f32)>,
    depth: u32,
    biomes: &'registry BiomeRegistry,
) -> Option<&'registry str> {
    influences
        .filter_map(|(biome_id, weight)| {
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));
            biome
                .surface_block_at_depth(depth)
                .map(|block_id| (block_id, weight))
        })
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(block_id, _)| block_id)
}
