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
const ISLAND_EDGE_BLEND: f32 = 0.16;
const ISLAND_TOP_BLEND: f32 = 0.020;
const ISLAND_BOTTOM_BLEND: f32 = 0.16;
const SECONDARY_LOBE_MIN_COUNT: usize = 3;
const SECONDARY_LOBE_VARIATION: usize = 3;

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
        DensityModifierContext {
            current_density,
            position,
            local_position: selection.local_position,
            seed,
            cave_depth_strength,
            allow_caverns,
            allow_solids,
        },
    ) * selection.strength
}

pub(crate) fn volume_biome_surface_depth(
    position: Vec3,
    selection: VolumeBiomeSelection,
    biome_field: &BiomeField,
) -> Option<u32> {
    let (modifier, seed) = biome_field.volume_density_modifier(selection)?;
    let BiomeDensityModifier::FloatingIsland {
        noise_scale,
        edge_irregularity,
        top_roughness,
        ..
    } = modifier
    else {
        return None;
    };

    let sample = floating_island_sample(
        position,
        selection.local_position,
        seed,
        noise_scale,
        edge_irregularity,
        top_roughness,
    );
    if sample.mask <= 0.0 || !sample.top.is_finite() {
        return None;
    }

    let depth_blocks =
        ((sample.top - selection.local_position.y).max(0.0) * selection.vertical_radius).floor();
    Some(depth_blocks.clamp(0.0, u32::MAX as f32) as u32)
}

#[derive(Clone, Copy)]
struct DensityModifierContext {
    current_density: f32,
    position: Vec3,
    local_position: Vec3,
    seed: u64,
    cave_depth_strength: f32,
    allow_caverns: bool,
    allow_solids: bool,
}

fn density_modifier_delta(
    modifier: BiomeDensityModifier,
    context: DensityModifierContext,
) -> f32 {
    let DensityModifierContext {
        current_density,
        position,
        local_position,
        seed,
        cave_depth_strength,
        allow_caverns,
        allow_solids,
    } = context;

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
            let sample = floating_island_sample(
                position,
                local_position,
                seed,
                noise_scale,
                edge_irregularity,
                top_roughness,
            );
            fill_density_delta(current_density, sample.mask, fill_margin)
        }
    }
}

fn fill_density_delta(current_density: f32, mask: f32, solid_margin: f32) -> f32 {
    if mask <= 0.0 {
        return 0.0;
    }

    ((-current_density).max(0.0) + solid_margin) * mask.clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug)]
struct FloatingIslandSample {
    mask: f32,
    top: f32,
}

#[derive(Clone, Copy, Debug)]
struct FloatingIslandLobe {
    center: Vec2,
    radii: Vec2,
    top_offset: f32,
    underside_depth: f32,
}

fn floating_island_sample(
    position: Vec3,
    local_position: Vec3,
    seed: u64,
    noise_scale: f32,
    edge_irregularity: f32,
    top_roughness: f32,
) -> FloatingIslandSample {
    let planar_noise = value_noise_3d(
        Vec3::new(position.x * noise_scale, 0.0, position.z * noise_scale),
        seed.rotate_left(17),
    );
    let detail_noise = value_noise_3d(
        Vec3::new(
            position.x * noise_scale * 2.35,
            11.0,
            position.z * noise_scale * 2.35,
        ),
        seed.rotate_left(43),
    );
    let edge_scale = (1.0 + planar_noise * edge_irregularity + detail_noise * edge_irregularity * 0.3)
        .max(0.62);
    let shared_top = 0.24 + planar_noise * top_roughness + detail_noise * top_roughness * 0.22;

    let mut union_mask = 0.0_f32;
    let mut highest_top = f32::NEG_INFINITY;
    let lobe_count = 1 + secondary_lobe_count(seed);

    for index in 0..lobe_count {
        let lobe = floating_island_lobe(seed, index);
        let delta = local_position.xz() - lobe.center;
        let scaled = Vec2::new(
            delta.x / (lobe.radii.x * edge_scale),
            delta.y / (lobe.radii.y * edge_scale),
        );
        let radial = scaled.length();
        if radial >= 1.0 + ISLAND_EDGE_BLEND {
            continue;
        }

        let top = shared_top + lobe.top_offset;
        highest_top = highest_top.max(top);
        let core = (1.0 - radial).clamp(0.0, 1.0);
        let bottom = top - (0.16 + lobe.underside_depth * core.powf(0.72));
        let edge_mask = smoothstep(
            ((1.0 + ISLAND_EDGE_BLEND - radial) / (ISLAND_EDGE_BLEND * 2.0))
                .clamp(0.0, 1.0),
        );
        let top_mask =
            smoothstep(((top - local_position.y) / ISLAND_TOP_BLEND).clamp(0.0, 1.0));
        let bottom_mask = smoothstep(
            ((local_position.y - bottom) / ISLAND_BOTTOM_BLEND).clamp(0.0, 1.0),
        );
        let lobe_mask = edge_mask * top_mask * bottom_mask;

        // Probabilistic-union form is a smooth max for [0, 1] masks. Because
        // every secondary lobe overlaps the central core, this creates one
        // connected island with gradual joins instead of separate stone blobs.
        union_mask = 1.0 - (1.0 - union_mask) * (1.0 - lobe_mask);
    }

    FloatingIslandSample {
        mask: union_mask.clamp(0.0, 1.0),
        top: highest_top,
    }
}

fn secondary_lobe_count(seed: u64) -> usize {
    SECONDARY_LOBE_MIN_COUNT
        + (hash_unit(seed, 0x6a09_e667_f3bc_c909) * SECONDARY_LOBE_VARIATION as f32)
            .floor()
            .clamp(0.0, (SECONDARY_LOBE_VARIATION - 1) as f32) as usize
}

fn floating_island_lobe(seed: u64, index: usize) -> FloatingIslandLobe {
    if index == 0 {
        return FloatingIslandLobe {
            center: Vec2::ZERO,
            radii: Vec2::new(0.64, 0.60),
            top_offset: 0.0,
            underside_depth: 1.35,
        };
    }

    let salt = index as u64;
    let angle = std::f32::consts::TAU * hash_unit(seed, 0x9e37_79b9_7f4a_7c15 ^ salt);
    let offset = 0.28 + 0.16 * hash_unit(seed, 0xc2b2_ae3d_27d4_eb4f ^ salt.rotate_left(7));
    let center = Vec2::new(angle.cos(), angle.sin()) * offset;
    let radius_x = 0.34 + 0.18 * hash_unit(seed, 0x1656_67b1_9e37_79f9 ^ salt.rotate_left(13));
    let radius_z = 0.34 + 0.18 * hash_unit(seed, 0x85eb_ca77_c2b2_ae63 ^ salt.rotate_left(19));
    let top_offset =
        (hash_unit(seed, 0x27d4_eb2f_1656_67c5 ^ salt.rotate_left(29)) - 0.5) * 0.10;
    let underside_depth =
        0.90 + 0.30 * hash_unit(seed, 0x94d0_49bb_1331_11eb ^ salt.rotate_left(37));

    FloatingIslandLobe {
        center,
        radii: Vec2::new(radius_x, radius_z),
        top_offset,
        underside_depth,
    }
}

fn hash_unit(seed: u64, salt: u64) -> f32 {
    let mut value = seed ^ salt;
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    value ^= value >> 33;
    ((value >> 40) as u32 as f32) / ((1_u32 << 24) - 1) as f32
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

    fn modifier_context(
        current_density: f32,
        position: Vec3,
        local_position: Vec3,
        seed: u64,
        cave_depth_strength: f32,
        allow_caverns: bool,
        allow_solids: bool,
    ) -> DensityModifierContext {
        DensityModifierContext {
            current_density,
            position,
            local_position,
            seed,
            cave_depth_strength,
            allow_caverns,
            allow_solids,
        }
    }
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
            modifier_context(current_density, position, Vec3::ZERO, 7, 1.0, true, true),
        );
        let solid = density_modifier_delta(
            BiomeDensityModifier::Solid {
                fill_strength: 20.0,
                noise_scale: 0.01,
                coverage: 1.0,
            },
            modifier_context(current_density, position, Vec3::ZERO, 7, 1.0, true, true),
        );

        assert!(current_density + cavern < 0.0);
        assert_eq!(solid, 20.0);
    }

    #[test]
    fn floating_island_is_connected_across_secondary_lobes() {
        let seed = 7;
        let secondary = floating_island_lobe(seed, 1);
        let bridge = secondary.center * 0.5;
        let bridge_sample = floating_island_sample(
            Vec3::new(bridge.x * 64.0, 0.0, bridge.y * 64.0),
            Vec3::new(bridge.x, 0.0, bridge.y),
            seed,
            0.03,
            0.0,
            0.0,
        );
        let secondary_sample = floating_island_sample(
            Vec3::new(secondary.center.x * 64.0, 0.0, secondary.center.y * 64.0),
            Vec3::new(secondary.center.x, 0.0, secondary.center.y),
            seed,
            0.03,
            0.0,
            0.0,
        );

        assert!(bridge_sample.mask > 0.35);
        assert!(secondary_sample.mask > 0.5);
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
            modifier_context(
                density,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 0.0),
                7,
                1.0,
                true,
                true,
            ),
        );
        let center_lower = density_modifier_delta(
            modifier,
            modifier_context(
                density,
                Vec3::ZERO,
                Vec3::new(0.0, -0.72, 0.0),
                7,
                1.0,
                true,
                true,
            ),
        );
        let edge_lower = density_modifier_delta(
            modifier,
            modifier_context(
                density,
                Vec3::ZERO,
                Vec3::new(1.18, -0.72, 0.0),
                7,
                1.0,
                true,
                true,
            ),
        );

        assert!(density + center_top > 0.0);
        assert!(density + center_lower > 0.0);
        assert!(density + edge_lower < 0.0);
    }

    #[test]
    fn floating_island_edge_tapers_before_the_outer_boundary() {
        let seed = 7;
        let inner = floating_island_sample(
            Vec3::ZERO,
            Vec3::new(0.84, 0.0, 0.0),
            seed,
            0.03,
            0.0,
            0.0,
        );
        let boundary = floating_island_sample(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            seed,
            0.03,
            0.0,
            0.0,
        );

        assert!(inner.mask > boundary.mask);
        assert!(boundary.mask > 0.0);
    }

    #[test]
    fn floating_island_top_layer_depth_stays_within_the_surface_block() {
        let top = 0.24;
        let local_y = top - (0.64 / 36.0);
        let depth =
            ((top - local_y).max(0.0) * 36.0).floor() as u32;

        assert_eq!(depth, 0);
    }

    #[test]
    fn floating_island_lobes_are_seeded_but_deterministic() {
        assert_eq!(
            floating_island_lobe(42, 2).center,
            floating_island_lobe(42, 2).center
        );
        assert_ne!(
            floating_island_lobe(42, 2).center,
            floating_island_lobe(43, 2).center
        );
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
            modifier_context(
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
            ),
        );

        assert_eq!(cavern, 0.0);
    }
}
