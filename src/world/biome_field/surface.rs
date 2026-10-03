use arrayvec::ArrayVec;
use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES,
    constants::BORDER_TRANSITION_WIDTH,
    spatial::{smoothstep, varied_surface_margin_width},
};

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if let Some(index) = self.single_surface_biome {
            return single_biome_sample(self, index);
        }

        let field_sample = self.surface_field_sample_at(position);
        let primary_index = field_sample.primary_index;
        debug_assert!(self.surface_biome_is_enabled(primary_index));
        let regional_boundary = field_sample
            .boundary
            .filter(|boundary| boundary.distance <= maximum_boundary_interest_radius(self));

        let mut influences = ArrayVec::<BiomeInfluence<'_>, MAX_SURFACE_INFLUENCES>::new();
        let neighbor_weight = regional_boundary
            .filter(|boundary| boundary.distance <= BORDER_TRANSITION_WIDTH)
            .map(|boundary| {
                let progress = 1.0 - (boundary.distance / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
                (
                    boundary.neighbor_surface_index,
                    smoothstep(progress),
                    boundary.neighbor_terrain_strength,
                )
            });
        let total_weight = 1.0 + neighbor_weight.map_or(0.0, |(_, weight, _)| weight);
        let primary = &self.surface_biomes[primary_index];
        influences.push(BiomeInfluence {
            id: primary.id.as_str(),
            weight: 1.0 / total_weight,
            surface_index: primary_index,
            terrain_strength: field_sample.primary_terrain_strength,
        });
        if let Some((neighbor_index, weight, terrain_strength)) = neighbor_weight
            && weight > 0.0
        {
            let neighbor = &self.surface_biomes[neighbor_index];
            influences.push(BiomeInfluence {
                id: neighbor.id.as_str(),
                weight: weight / total_weight,
                surface_index: neighbor_index,
                terrain_strength,
            });
        }

        let surface_margin_index = regional_boundary.and_then(|boundary| {
            let margin_owner = &self.surface_biomes[boundary.neighbor_surface_index];
            let margin = margin_owner.surface_margin?;
            let width = varied_surface_margin_width(
                position,
                margin.noise_seed,
                margin.width,
                margin.width_variation,
                margin.variation_scale,
            );
            (boundary.distance <= width).then_some(boundary.neighbor_surface_index)
        });
        let identity_surface_index = surface_margin_index.unwrap_or(primary_index);

        BiomeFieldSample {
            primary_id: self.surface_biomes[identity_surface_index].id.as_str(),
            primary_surface_index: primary_index,
            surface_margin_index,
            identity_surface_index,
            influences,
        }
    }
}

fn single_biome_sample(field: &BiomeField, index: usize) -> BiomeFieldSample<'_> {
    let biome = &field.surface_biomes[index];
    let mut influences = ArrayVec::new();
    influences.push(BiomeInfluence {
        id: biome.id.as_str(),
        weight: 1.0,
        surface_index: index,
        terrain_strength: 1.0,
    });
    BiomeFieldSample {
        primary_id: biome.id.as_str(),
        primary_surface_index: index,
        surface_margin_index: None,
        identity_surface_index: index,
        influences,
    }
}

fn maximum_boundary_interest_radius(field: &BiomeField) -> f32 {
    field
        .surface_biomes
        .iter()
        .filter_map(|biome| biome.surface_margin)
        .map(|margin| margin.width + margin.width_variation.abs())
        .fold(BORDER_TRANSITION_WIDTH, f32::max)
        .max(0.0)
}
