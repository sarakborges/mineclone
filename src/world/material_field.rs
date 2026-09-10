use crate::content::{biome::BiomeRegistry, block_id::intern_block_id};

use super::{
    biome_field::{BiomeField, BiomeFieldSample, VolumeBiomeSelection},
    geology::GeologyRegion,
    hydrology::HydrologyRegion,
};

pub(crate) fn solid_block_id(
    position: bevy::prelude::Vec3,
    surface: &BiomeFieldSample<'_>,
    volume: Option<VolumeBiomeSelection>,
    biome_field: &BiomeField,
    geology: &GeologyRegion,
    hydrology: &HydrologyRegion,
    biomes: &BiomeRegistry,
    fallback: &'static str,
) -> &'static str {
    if let Some(block_id) = volume.and_then(|selection| biome_field.volume_solid_block(selection)) {
        return intern_block_id(block_id);
    }

    if let Some(block_id) = geology.solid_block_at(position) {
        return intern_block_id(block_id);
    }

    if let Some(block_id) = hydrology.solid_block_at(position) {
        return intern_block_id(block_id);
    }

    strongest_material(
        surface
            .influences
            .iter()
            .map(|influence| (influence.id, influence.weight)),
        biomes,
    )
    .map(intern_block_id)
    .unwrap_or(fallback)
}

fn strongest_material<'registry, 'id>(
    influences: impl Iterator<Item = (&'id str, f32)>,
    biomes: &'registry BiomeRegistry,
) -> Option<&'registry str> {
    influences
        .filter_map(|(biome_id, weight)| {
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));
            biome
                .solid_block
                .as_deref()
                .map(|block_id| (block_id, weight))
        })
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(block_id, _)| block_id)
}
