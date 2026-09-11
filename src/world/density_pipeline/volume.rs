use bevy::prelude::*;

use crate::{
    content::biome_density::BiomeDensityModifier,
    world::biome_field::{BiomeField, VolumeBiomeSelection},
};

use super::{carve_density_delta, noise::value_noise_3d};

const DENSITY_NOISE_EDGE: f32 = 0.15;

pub(super) fn volume_biome_density_delta(
    current_density: f32,
    position: Vec3,
    volume: Option<VolumeBiomeSelection>,
    biome_field: &BiomeField,
    cave_depth_strength: f32,
) -> f32 {
    let Some(selection) = volume else {
        return 0.0;
    };
    let Some((modifier, seed)) = biome_field.volume_density_modifier(selection) else {
        return 0.0;
    };

    density_modifier_delta(
        modifier,
        current_density,
        position,
        seed,
        cave_depth_strength,
    ) * selection.strength
}

fn density_modifier_delta(
    modifier: BiomeDensityModifier,
    current_density: f32,
    position: Vec3,
    seed: u64,
    cave_depth_strength: f32,
) -> f32 {
    match modifier {
        BiomeDensityModifier::Cavern {
            carve_strength,
            noise_scale,
            openness,
        } => {
            if cave_depth_strength <= 0.0 {
                return 0.0;
            }

            let noise = value_noise_3d(position * noise_scale, seed) * 0.5 + 0.5;
            let mask = coverage_mask(noise, openness) * cave_depth_strength;
            carve_density_delta(current_density, mask, carve_strength)
        }
        BiomeDensityModifier::Solid {
            fill_strength,
            noise_scale,
            coverage,
        } => {
            let noise = value_noise_3d(position * noise_scale, seed) * 0.5 + 0.5;
            let mask = coverage_mask(noise, coverage);
            fill_strength * mask
        }
    }
}

fn coverage_mask(noise: f32, coverage: f32) -> f32 {
    if coverage <= 0.0 {
        return 0.0;
    }

    if coverage >= 1.0 {
        return 1.0;
    }

    let threshold = 1.0 - coverage;
    smoothstep(((noise - threshold) / DENSITY_NOISE_EDGE).clamp(0.0, 1.0))
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::density_pipeline::{
        CAVERN_FULL_STRENGTH_SURFACE_DEPTH, CAVERN_MINIMUM_SURFACE_DEPTH, depth_strength,
    };

    #[test]
    fn cavern_and_solid_modifiers_move_density_in_opposite_directions() {
        let position = Vec3::new(12.5, 30.5, -8.5);
        let current_density = 20.0;
        let cavern = density_modifier_delta(
            BiomeDensityModifier::Cavern {
                carve_strength: 4.0,
                noise_scale: 0.01,
                openness: 1.0,
            },
            current_density,
            position,
            7,
            1.0,
        );
        let solid = density_modifier_delta(
            BiomeDensityModifier::Solid {
                fill_strength: 20.0,
                noise_scale: 0.01,
                coverage: 1.0,
            },
            current_density,
            position,
            7,
            1.0,
        );

        assert!(current_density + cavern < 0.0);
        assert_eq!(solid, 20.0);
    }

    #[test]
    fn cavern_modifier_cannot_open_shallow_terrain_without_an_entrance_connector() {
        let position = Vec3::new(12.5, 70.5, -8.5);
        let current_density = 8.0;
        let cavern = density_modifier_delta(
            BiomeDensityModifier::Cavern {
                carve_strength: 4.0,
                noise_scale: 0.01,
                openness: 1.0,
            },
            current_density,
            position,
            7,
            depth_strength(
                current_density,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
        );

        assert_eq!(cavern, 0.0);
    }
}
