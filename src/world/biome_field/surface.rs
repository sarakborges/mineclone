use arrayvec::ArrayVec;
use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES, SurfaceBoundarySample,
    constants::{BORDER_TRANSITION_WIDTH, VISUAL_BLEND_WIDTH},
    spatial::{smoothstep, varied_surface_margin_width},
};

#[derive(Clone, Copy)]
struct ResolvedSurfaceBoundary {
    neighbor_surface_index: usize,
    neighbor_terrain_strength: f32,
    distance: f32,
    uses_separator: bool,
}

#[derive(Clone, Copy)]
struct SurfaceBoundaryBlend {
    neighbor_surface_index: usize,
    neighbor_terrain_strength: f32,
    primary_weight: f32,
    neighbor_weight: f32,
    uses_separator: bool,
}

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        self.sample_surface_with_transition_width(position, BORDER_TRANSITION_WIDTH)
    }

    pub(crate) fn sample_visual_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        self.sample_surface_with_transition_width(position, VISUAL_BLEND_WIDTH)
    }

    fn sample_surface_with_transition_width(
        &self,
        position: Vec2,
        transition_width: f32,
    ) -> BiomeFieldSample<'_> {
        if let Some(index) = self.single_surface_biome {
            return single_biome_sample(self, index);
        }

        let field_sample = self.surface_field_sample_at(position);
        let raw_primary_index = field_sample.primary_index;
        debug_assert!(self.surface_biome_is_enabled(raw_primary_index));
        let regional_boundary = field_sample
            .boundary
            .filter(|boundary| {
                boundary.distance <= maximum_boundary_interest_radius(self, transition_width)
            })
            .map(|boundary| self.resolve_surface_boundary(raw_primary_index, boundary));
        let boundary_blend = regional_boundary
            .filter(|boundary| boundary.distance <= transition_width)
            .map(|boundary| surface_boundary_blend(boundary, transition_width));

        let primary_surface_index = boundary_blend
            .filter(|blend| {
                blend.uses_separator && blend.neighbor_weight > blend.primary_weight
            })
            .map_or(raw_primary_index, |blend| blend.neighbor_surface_index);

        let mut influences = ArrayVec::<BiomeInfluence<'_>, MAX_SURFACE_INFLUENCES>::new();
        if let Some(blend) = boundary_blend {
            push_surface_influence(
                &mut influences,
                &self.surface_biomes[raw_primary_index],
                raw_primary_index,
                blend.primary_weight,
                field_sample.primary_terrain_strength,
            );
            push_surface_influence(
                &mut influences,
                &self.surface_biomes[blend.neighbor_surface_index],
                blend.neighbor_surface_index,
                blend.neighbor_weight,
                blend.neighbor_terrain_strength,
            );
        } else {
            push_surface_influence(
                &mut influences,
                &self.surface_biomes[raw_primary_index],
                raw_primary_index,
                1.0,
                field_sample.primary_terrain_strength,
            );
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
        let identity_surface_index = surface_margin_index.unwrap_or(primary_surface_index);

        BiomeFieldSample {
            primary_id: self.surface_biomes[identity_surface_index].id.as_str(),
            primary_surface_index,
            surface_margin_index,
            identity_surface_index,
            influences,
        }
    }

    fn resolve_surface_boundary(
        &self,
        primary_index: usize,
        boundary: SurfaceBoundarySample,
    ) -> ResolvedSurfaceBoundary {
        if self.surface_biomes_can_neighbor(primary_index, boundary.neighbor_surface_index) {
            return ResolvedSurfaceBoundary {
                neighbor_surface_index: boundary.neighbor_surface_index,
                neighbor_terrain_strength: boundary.neighbor_terrain_strength,
                distance: boundary.distance,
                uses_separator: false,
            };
        }

        let separator = self
            .surface_boundary_separator(primary_index, boundary.neighbor_surface_index)
            .unwrap_or_else(|| {
                panic!(
                    "surface biomes {} and {} deny adjacency but no compatible separator biome exists",
                    self.surface_biome_id(primary_index),
                    self.surface_biome_id(boundary.neighbor_surface_index),
                )
            });

        ResolvedSurfaceBoundary {
            neighbor_surface_index: separator,
            neighbor_terrain_strength: 1.0,
            distance: boundary.distance,
            uses_separator: true,
        }
    }
}

fn surface_boundary_blend(
    boundary: ResolvedSurfaceBoundary,
    transition_width: f32,
) -> SurfaceBoundaryBlend {
    let width = transition_width.max(f32::MIN_POSITIVE);
    let progress = 1.0 - (boundary.distance / width).clamp(0.0, 1.0);
    let transition = smoothstep(progress);
    let (primary_weight, neighbor_weight) = if boundary.uses_separator {
        // An authored-incompatible pair must never meet directly. Drive the
        // separator all the way to full ownership at the raw field boundary so
        // both sides converge on the same biome instead of swapping abruptly.
        (1.0 - transition, transition)
    } else {
        // Compatible regions meet at an even 50/50 blend because primary
        // ownership flips on the other side of the same raw field boundary.
        let total = 1.0 + transition;
        (1.0 / total, transition / total)
    };

    SurfaceBoundaryBlend {
        neighbor_surface_index: boundary.neighbor_surface_index,
        neighbor_terrain_strength: boundary.neighbor_terrain_strength,
        primary_weight,
        neighbor_weight,
        uses_separator: boundary.uses_separator,
    }
}

fn push_surface_influence<'a>(
    influences: &mut ArrayVec<BiomeInfluence<'a>, MAX_SURFACE_INFLUENCES>,
    biome: &'a super::BiomeFieldEntry,
    surface_index: usize,
    weight: f32,
    terrain_strength: f32,
) {
    if weight <= f32::EPSILON {
        return;
    }
    influences.push(BiomeInfluence {
        id: biome.id.as_str(),
        weight,
        surface_index,
        terrain_strength,
    });
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

fn maximum_boundary_interest_radius(field: &BiomeField, transition_width: f32) -> f32 {
    field
        .surface_biomes
        .iter()
        .filter_map(|biome| biome.surface_margin)
        .map(|margin| margin.width + margin.width_variation.abs())
        .fold(transition_width, f32::max)
        .max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary(uses_separator: bool, distance: f32) -> ResolvedSurfaceBoundary {
        ResolvedSurfaceBoundary {
            neighbor_surface_index: 1,
            neighbor_terrain_strength: 1.0,
            distance,
            uses_separator,
        }
    }

    #[test]
    fn compatible_boundary_meets_at_equal_weights() {
        let blend = surface_boundary_blend(boundary(false, 0.0), 32.0);
        assert!((blend.primary_weight - 0.5).abs() < f32::EPSILON);
        assert!((blend.neighbor_weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn incompatible_boundary_converges_to_separator() {
        let blend = surface_boundary_blend(boundary(true, 0.0), 32.0);
        assert!(blend.primary_weight <= f32::EPSILON);
        assert!((blend.neighbor_weight - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn separator_fades_out_at_transition_edge() {
        let blend = surface_boundary_blend(boundary(true, 32.0), 32.0);
        assert!((blend.primary_weight - 1.0).abs() < f32::EPSILON);
        assert!(blend.neighbor_weight <= f32::EPSILON);
    }
}
