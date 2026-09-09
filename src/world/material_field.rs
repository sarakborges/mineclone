use crate::content::biome::BiomeRegistry;

use super::biome_field::{BiomeFieldSample, VolumeBiomeFieldSample};

pub(crate) fn solid_block_id<'a>(
    surface: &BiomeFieldSample<'_>,
    volume: Option<&VolumeBiomeFieldSample<'_>>,
    biomes: &'a BiomeRegistry,
    fallback: &'a str,
) -> &'a str {
    if let Some(volume) = volume {
        if let Some(block_id) = strongest_material(volume.influences.iter().map(|influence| {
            (influence.id, influence.weight)
        }), biomes)
        {
            return block_id;
        }
    }

    strongest_material(
        surface
            .influences
            .iter()
            .map(|influence| (influence.id, influence.weight)),
        biomes,
    )
    .unwrap_or(fallback)
}

fn strongest_material<'a>(
    influences: impl Iterator<Item = (&'a str, f32)>,
    biomes: &'a BiomeRegistry,
) -> Option<&'a str> {
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
