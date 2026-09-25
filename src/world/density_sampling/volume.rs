use bevy::prelude::*;

use crate::{
    content::biome_density::BiomeDensityModifier,
    world::{
        biome_field::{BiomeField, VolumeBiomeSelection},
        math::smoothstep,
        noise::value_noise_3d,
    },
};

use super::carve_density_delta;

const DENSITY_NOISE_EDGE: f32 = 0.15;

pub(super) fn volume_biome_density_delta(
    current_density: f32,
    position: Vec3,
    volume: Option<VolumeBiomeSelection>,
    biome_field: &BiomeField,
    cave_depth_strength: f32,
    allow_caverns: bool,
    allow_solids: bool,
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
        selection.local_position,
        seed,
        cave_depth_strength,
        allow_caverns,
        allow_solids,
    ) * selection.strength
}

fn density_modifier_delta(
    modifier: BiomeDensityModifier,
    current_density: f32,
    position: Vec3,
    local_position: Vec3,
    seed: u64,
    cave_depth_strength: f32,
    allow_caverns: bool,
    allow_solids: bool,
) -> f32 {
    match modifier {
        BiomeDensityModifier::Cavern {
            carve_strength,
            noise_scale,
            openness,
        } => {
            if !allow_caverns || cave_depth_strength <= 0.0 {
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
            if !allow_solids {
                return 0.0;
            }
            let noise = value_noise_3d(position * noise_scale, seed) * 0.5 + 0.5;
            let mask = coverage_mask(noise, coverage);
            fill_strength * mask
        }
        BiomeDensityModifier::FloatingIsland {
            fill_margin,
            noise_scale,
            edge_irregularity,
            top_roughness,
        } => {
            if !allow_solids {
                return 0.0;
            }
            let mask = floating_island_mask(
                position,
                local_position,
                seed,
                noise_scale,
                edge_irregularity,
                top_roughness,
            );
            fill_density_delta(current_density, mask, fill_margin)
        }
    }
}

fn fill_density_delta(current_density: f32, mask: f32, solid_margin: f32) -> f32 {
    if mask <= 0.0 {
        return 0.0;
    }

    ((-current_density).max(0.0) + solid_margin) * mask.clamp(0.0, 1.0)
}

fn floating_island_mask(
    position: Vec3,
    local_position: Vec3,
    seed: u64,
    noise_scale: f32,
    edge_irregularity: f32,
    top_roughness: f32,
) -> f32 {
    let planar_noise = value_noise_3d(
        Vec3::new(position.x * noise_scale, 0.0, position.z * noise_scale),
        seed.rotate_left(17),
    );
    let edge_radius = (1.0 + planar_noise * edge_irregularity).max(0.55);
    let radial = Vec2::new(local_position.x, local_position.z).length() / edge_radius;
    if radial >= 1.08 {
        return 0.0;
    }

    let top = 0.22 + planar_noise * top_roughness;
    let core = (1.0 - radial).clamp(0.0, 1.0);
    // Thin shoreline, broad upper mass, and a pointed underside at the center.
    let bottom = top - (0.18 + 1.05 * core.powf(0.7));

    let edge_mask = smoothstep(((1.0 - radial) / 0.08).clamp(0.0, 1.0));
    let top_mask = smoothstep(((top - local_position.y) / 0.08).clamp(0.0, 1.0));
    let bottom_mask =
        smoothstep(((local_position.y - bottom) / 0.10).clamp(0.0, 1.0));

    edge_mask * top_mask * bottom_mask
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::density_sampling::{
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
            Vec3::ZERO,
            7,
            1.0,
            true,
            true,
        );
        let solid = density_modifier_delta(
            BiomeDensityModifier::Solid {
                fill_strength: 20.0,
                noise_scale: 0.01,
                coverage: 1.0,
            },
            current_density,
            position,
            Vec3::ZERO,
            7,
            1.0,
            true,
            true,
        );

        assert!(current_density + cavern < 0.0);
        assert_eq!(solid, 20.0);
    }

    #[test]
    fn floating_island_has_a_broad_top_and_tapered_underside() {
        let modifier = BiomeDensityModifier::FloatingIsland {
            fill_margin: 4.0,
            noise_scale: 0.03,
            edge_irregularity: 0.0,
            top_roughness: 0.0,
        };
        let density = -80.0;

        let center_top = density_modifier_delta(
            modifier,
            density,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 0.0),
            7,
            1.0,
            true,
            true,
        );
        let center_lower = density_modifier_delta(
            modifier,
            density,
            Vec3::ZERO,
            Vec3::new(0.0, -0.72, 0.0),
            7,
            1.0,
            true,
            true,
        );
        let edge_lower = density_modifier_delta(
            modifier,
            density,
            Vec3::ZERO,
            Vec3::new(0.92, -0.72, 0.0),
            7,
            1.0,
            true,
            true,
        );

        assert!(density + center_top > 0.0);
        assert!(density + center_lower > 0.0);
        assert!(density + edge_lower < 0.0);
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
            Vec3::ZERO,
            7,
            depth_strength(
                current_density,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
            true,
            true,
        );

        assert_eq!(cavern, 0.0);
    }
}
