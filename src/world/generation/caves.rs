mod anchors;
mod entrances;

use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, biome_density::BiomeDensityModifier, dimension::DimensionDefinition,
    },
    world::{
        biome_field::BiomeField,
        cave_connectivity::CaveConnectivityRegion,
        generation_region::{GenerationRegion, generation_region_world_bounds},
        world_feature_fields::WorldFeatureFields,
    },
};

use self::{
    anchors::resolve_open_cavern_anchor,
    entrances::{ocean_cave_entrance, surface_cave_entrance},
};

pub(super) fn anchored_cave_region(
    region: &GenerationRegion,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
    dimension: &DimensionDefinition,
    feature_fields: &WorldFeatureFields,
) -> Option<Arc<CaveConnectivityRegion>> {
    feature_fields.cave_region(region.coord, |cave_field| {
        let (minimum, maximum) = generation_region_world_bounds(region.coord);
        let margin = Vec3::splat(cave_field.anchor_search_margin());
        let search_minimum = minimum - margin;
        let search_maximum = maximum + margin;
        let volume_region = biome_field.volume_region_in_bounds(search_minimum, search_maximum);
        let mut anchors = biome_field
            .volume_anchors_in_region(&volume_region)
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
            .filter_map(|anchor| {
                resolve_open_cavern_anchor(
                    anchor.position,
                    region,
                    &volume_region,
                    dimension,
                    biomes,
                    biome_field,
                )
            })
            .collect::<Vec<_>>();
        let underground_anchors = anchors.clone();
        let mut water_source_anchors = Vec::new();

        if region.coord.y == 0 {
            if let Some(entrance) = surface_cave_entrance(
                region,
                &underground_anchors,
                minimum,
                maximum,
                dimension,
                biomes,
                biome_field,
            ) {
                anchors.push(entrance);
            }

            if let Some(ocean_opening) = ocean_cave_entrance(
                region,
                &underground_anchors,
                minimum,
                maximum,
                biome_field,
            ) {
                anchors.push(ocean_opening);
                water_source_anchors.push(ocean_opening);
            }
        }

        (!anchors.is_empty()).then(|| {
            cave_field.region_from_anchors_with_water_sources(
                region.coord,
                &anchors,
                &underground_anchors,
                &water_source_anchors,
            )
        })
    })
}
