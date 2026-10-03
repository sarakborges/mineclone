use arrayvec::ArrayVec;
use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES, SurfaceBoundarySample,
    constants::BORDER_TRANSITION_WIDTH,
    spatial::{smoothstep, varied_surface_margin_width},
};

const EDGE_PROBE_STEP: f32 = 4.0;
const EDGE_PROBE_DIRECTION_PADDING: f32 = 1.1;
const EDGE_REFINEMENT_STEPS: usize = 6;
const DIAGONAL: f32 = std::f32::consts::FRAC_1_SQRT_2;
const EDGE_PROBE_DIRECTIONS: [Vec2; 8] = [
    Vec2::X,
    Vec2::NEG_X,
    Vec2::Y,
    Vec2::NEG_Y,
    Vec2::new(DIAGONAL, DIAGONAL),
    Vec2::new(DIAGONAL, -DIAGONAL),
    Vec2::new(-DIAGONAL, DIAGONAL),
    Vec2::new(-DIAGONAL, -DIAGONAL),
];

impl BiomeField {
    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if let Some(index) = self.single_surface_biome {
            return single_biome_sample(self, index);
        }

        let primary_index = self.surface_biome_index_at(position);
        let regional_boundary = nearest_surface_boundary(
            self,
            position,
            primary_index,
            maximum_boundary_interest_radius(self),
        );

        let mut influences = ArrayVec::<BiomeInfluence<'_>, MAX_SURFACE_INFLUENCES>::new();
        let neighbor_weight = regional_boundary
            .filter(|boundary| boundary.distance <= BORDER_TRANSITION_WIDTH)
            .map(|boundary| {
                let progress =
                    1.0 - (boundary.distance / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
                (boundary.neighbor_surface_index, smoothstep(progress))
            });
        let total_weight = 1.0 + neighbor_weight.map_or(0.0, |(_, weight)| weight);
        let primary = &self.surface_biomes[primary_index];
        influences.push(BiomeInfluence {
            id: primary.id.as_str(),
            weight: 1.0 / total_weight,
            surface_index: primary_index,
            terrain_strength: 1.0,
        });
        if let Some((neighbor_index, weight)) = neighbor_weight
            && weight > 0.0
        {
            let neighbor = &self.surface_biomes[neighbor_index];
            influences.push(BiomeInfluence {
                id: neighbor.id.as_str(),
                weight: weight / total_weight,
                surface_index: neighbor_index,
                terrain_strength: 1.0,
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

fn nearest_surface_boundary(
    field: &BiomeField,
    position: Vec2,
    primary_index: usize,
    interest_radius: f32,
) -> Option<SurfaceBoundarySample> {
    if interest_radius <= f32::EPSILON {
        return None;
    }

    let probe_radius = interest_radius * EDGE_PROBE_DIRECTION_PADDING + EDGE_PROBE_STEP;
    let probe_steps = (probe_radius / EDGE_PROBE_STEP).ceil() as usize;
    let mut best: Option<SurfaceBoundarySample> = None;

    for direction in EDGE_PROBE_DIRECTIONS {
        let mut previous_distance = 0.0;
        for step in 1..=probe_steps {
            let distance = (step as f32 * EDGE_PROBE_STEP).min(probe_radius);
            let candidate_index =
                field.surface_biome_index_at(position + direction * distance);
            if candidate_index == primary_index {
                previous_distance = distance;
                continue;
            }

            let boundary = refine_surface_boundary(
                field,
                position,
                direction,
                primary_index,
                previous_distance,
                distance,
                candidate_index,
            );
            if boundary.distance <= interest_radius
                && best.is_none_or(|current| {
                    boundary.distance < current.distance
                        || (boundary.distance == current.distance
                            && boundary.neighbor_surface_index < current.neighbor_surface_index)
                })
            {
                best = Some(boundary);
            }
            break;
        }
    }

    best
}

fn refine_surface_boundary(
    field: &BiomeField,
    position: Vec2,
    direction: Vec2,
    primary_index: usize,
    mut lower: f32,
    mut upper: f32,
    mut neighbor_surface_index: usize,
) -> SurfaceBoundarySample {
    for _ in 0..EDGE_REFINEMENT_STEPS {
        let middle = (lower + upper) * 0.5;
        let candidate_index = field.surface_biome_index_at(position + direction * middle);
        if candidate_index == primary_index {
            lower = middle;
        } else {
            upper = middle;
            neighbor_surface_index = candidate_index;
        }
    }

    SurfaceBoundarySample {
        neighbor_surface_index,
        distance: upper,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_probe_directions_are_unit_vectors() {
        for direction in EDGE_PROBE_DIRECTIONS {
            assert!((direction.length() - 1.0).abs() <= 1e-6);
        }
    }

    #[test]
    fn direction_padding_covers_half_of_an_eight_way_sector() {
        let half_sector = std::f32::consts::FRAC_PI_8;
        assert!(EDGE_PROBE_DIRECTION_PADDING >= 1.0 / half_sector.cos());
    }
}
