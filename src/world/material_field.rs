use crate::content::{
    biome::BiomeRegistry,
    block::BlockRegistry,
};

use super::{
    biome_field::{BiomeFieldSample, VolumeBiomeFieldSample},
    geology::GeologyRegion,
};

pub(crate) fn solid_block_id(
    position: bevy::prelude::Vec3,
    surface: &BiomeFieldSample<'_>,
    volume: Option<&VolumeBiomeFieldSample<'_>>,
    geology: &GeologyRegion,
    biomes: &BiomeRegistry,
    blocks: &BlockRegistry,
    fallback: &'static str,
) -> &'static str {
    if let Some(volume) = volume {
        if let Some(block_id) = strongest_material(
            volume
                .influences
                .iter()
                .map(|influence| (influence.id, influence.weight)),
            biomes,
        ) {
            return blocks
                .static_id(block_id)
                .unwrap_or_else(|| panic!("biome references missing solid block: {block_id}"));
        }
    }

    if let Some(block_id) = geology.solid_block_at(position) {
        return blocks
            .static_id(block_id)
            .unwrap_or_else(|| panic!("geology references missing solid block: {block_id}"));
    }

    if let Some(block_id) = strongest_material(
        surface
            .influences
            .iter()
            .map(|influence| (influence.id, influence.weight)),
        biomes,
    ) {
        return blocks
            .static_id(block_id)
            .unwrap_or_else(|| panic!("biome references missing solid block: {block_id}"));
    }

    fallback
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
            biome.solid_block.as_deref().map(|block_id| (block_id, weight))
        })
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(block_id, _)| block_id)
}
