use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, biome_density::BiomeDensityModifier},
    world::{
        biome_field::BiomeField,
        cave_connectivity::CaveConnectivityRegion,
        generation_region::{GenerationRegion, generation_region_world_bounds},
        world_feature_fields::WorldFeatureFields,
    },
};

pub(super) fn anchored_cave_region(
    region: &GenerationRegion,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
    feature_fields: &WorldFeatureFields,
) -> Option<Arc<CaveConnectivityRegion>> {
    feature_fields.cave_region(region.coord, |cave_field| {
        let (minimum, maximum) = generation_region_world_bounds(region.coord);
        let margin = Vec3::splat(cave_field.anchor_search_margin());
        let anchors = biome_field
            .volume_anchors_in_bounds(minimum - margin, maximum + margin)
            .into_iter()
            .filter(|anchor| {
                let biome = biomes
                    .get(anchor.id)
                    .unwrap_or_else(|| panic!("missing biome definition: {}", anchor.id));
                matches!(
                    biome.density_modifier,
                    Some(BiomeDensityModifier::Cavern { .. })
                )
            })
            .map(|anchor| anchor.position)
            .collect::<Vec<_>>();

        (anchors.len() >= 2).then(|| cave_field.region_from_anchors(region.coord, &anchors))
    })
}
