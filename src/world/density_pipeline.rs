use bevy::prelude::*;

use crate::content::biome_density::BiomeDensityModifier;

use super::{
    biome_field::{BiomeField, VolumeBiomeSelection},
    cave_connectivity::CaveConnectivityRegion,
    generation_region::GenerationRegion,
};

const DENSITY_NOISE_EDGE: f32 = 0.15;
const CAVE_CONNECTOR_AIR_MARGIN: f32 = 5.5;
const CAVE_CONNECTOR_MINIMUM_SURFACE_DEPTH: f32 = -2.0;
const CAVE_CONNECTOR_FULL_STRENGTH_SURFACE_DEPTH: f32 = 1.5;
const CAVERN_MINIMUM_SURFACE_DEPTH: f32 = 12.0;
const CAVERN_FULL_STRENGTH_SURFACE_DEPTH: f32 = 20.0;
const CAVE_WATER_PROTECTION_DEPTH: f32 = 14.0;
const CAVE_WATER_PROTECTION_FADE_DEPTH: f32 = 20.0;
const CAVE_WATER_HORIZONTAL_CLEARANCE: f32 = 8.0;
const WATER_VOLUME_AIR_DENSITY: f32 = -0.001;

pub fn sample_density(
    base_density: f32,
    position: Vec3,
    region: &GenerationRegion,
    anchored_caves: Option<&CaveConnectivityRegion>,
    volume: Option<VolumeBiomeSelection>,
    biome_field: &BiomeField,
) -> f32 {
    let hydrology_delta = region.hydrology.density_delta(position);
    let geology_delta = region.geology.density_delta(position);
    let mut density = base_density + hydrology_delta + geology_delta;
    let water_clearance = cave_water_clearance(position, region);
    let connector_depth_strength = depth_strength(
        base_density,
        CAVE_CONNECTOR_MINIMUM_SURFACE_DEPTH,
        CAVE_CONNECTOR_FULL_STRENGTH_SURFACE_DEPTH,
    ) * water_clearance;

    if connector_depth_strength > 0.0
        && let Some(connector) = anchored_caves
            .and_then(|caves| caves.connector_graph.sample(position))
            .map(|sample| smoothstep(sample.strength) * connector_depth_strength)
    {
        density += carve_density_delta(density, connector, CAVE_CONNECTOR_AIR_MARGIN);
    }

    let cavern_depth_strength = depth_strength(
        base_density,
        CAVERN_MINIMUM_SURFACE_DEPTH,
        CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
    ) * water_clearance;

    density += volume_biome_density_delta(
        density,
        position,
        volume,
        biome_field,
        cavern_depth_strength,
    );

    enforce_hydrology_water_volume(density, base_density, position, region)
}

fn enforce_hydrology_water_volume(
    density: f32,
    base_density: f32,
    position: Vec3,
    region: &GenerationRegion,
) -> f32 {
    let horizontal = Vec2::new(position.x, position.z);
    let cell_bottom = position.y - 0.5;
    let cell_top = position.y + 0.5;

    if let Some(river) = region.hydrology.river_water_at(horizontal)
        && cell_top > river.bed_level
        && base_density > 0.0
    {
        return density.min(WATER_VOLUME_AIR_DENSITY);
    }

    let Some(water) = region.hydrology.water_at(horizontal) else {
        return density;
    };

    if cell_top <= water.bed_level || cell_bottom >= water.water_level {
        return density;
    }

    density.min(WATER_VOLUME_AIR_DENSITY)
}

fn cave_water_clearance(position: Vec3, region: &GenerationRegion) -> f32 {
    let horizontal = Vec2::new(position.x, position.z);
    let Some(water) = region
        .hydrology
        .water_near(horizontal, CAVE_WATER_HORIZONTAL_CLEARANCE)
    else {
        return 1.0;
    };
    let depth_below_water = water.water_level - position.y;

    if depth_below_water < 0.0 {
        return 1.0;
    }
    if depth_below_water <= CAVE_WATER_PROTECTION_DEPTH {
        return 0.0;
    }
    if depth_below_water >= CAVE_WATER_PROTECTION_FADE_DEPTH {
        return 1.0;
    }

    smoothstep(
        (depth_below_water - CAVE_WATER_PROTECTION_DEPTH)
            / (CAVE_WATER_PROTECTION_FADE_DEPTH - CAVE_WATER_PROTECTION_DEPTH),
    )
}

fn depth_strength(base_density: f32, minimum_depth: f32, full_strength_depth: f32) -> f32 {
    if base_density <= minimum_depth {
        return 0.0;
    }

    if base_density >= full_strength_depth {
        return 1.0;
    }

    let progress = (base_density - minimum_depth) / (full_strength_depth - minimum_depth);
    smoothstep(progress.clamp(0.0, 1.0))
}

fn carve_density_delta(density: f32, strength: f32, air_margin: f32) -> f32 {
    if strength <= 0.0 {
        return 0.0;
    }

    -(density.max(0.0) + air_margin) * strength.clamp(0.0, 1.0)
}

fn volume_biome_density_delta(
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

fn value_noise_3d(position: Vec3, seed: u64) -> f32 {
    let x0 = position.x.floor() as i32;
    let y0 = position.y.floor() as i32;
    let z0 = position.z.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;
    let tx = smoothstep(position.x - x0 as f32);
    let ty = smoothstep(position.y - y0 as f32);
    let tz = smoothstep(position.z - z0 as f32);

    let c000 = lattice_noise_3d(x0, y0, z0, seed);
    let c100 = lattice_noise_3d(x1, y0, z0, seed);
    let c010 = lattice_noise_3d(x0, y1, z0, seed);
    let c110 = lattice_noise_3d(x1, y1, z0, seed);
    let c001 = lattice_noise_3d(x0, y0, z1, seed);
    let c101 = lattice_noise_3d(x1, y0, z1, seed);
    let c011 = lattice_noise_3d(x0, y1, z1, seed);
    let c111 = lattice_noise_3d(x1, y1, z1, seed);

    let x00 = lerp(c000, c100, tx);
    let x10 = lerp(c010, c110, tx);
    let x01 = lerp(c001, c101, tx);
    let x11 = lerp(c011, c111, tx);
    let y0 = lerp(x00, x10, ty);
    let y1 = lerp(x01, x11, ty);

    lerp(y0, y1, tz)
}

fn lattice_noise_3d(x: i32, y: i32, z: i32, seed: u64) -> f32 {
    let mut hash = seed;
    hash ^= (x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (y as i64 as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= (z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    let normalized = (hash & 0xffff) as f32 / u16::MAX as f32;

    normalized * 2.0 - 1.0
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_explicit_cave_strength_opens_deep_solid_density() {
        let density = 80.0;
        let carved = density + carve_density_delta(density, 1.0, 4.0);

        assert!(carved < 0.0);
    }

    #[test]
    fn cavern_carving_stays_suppressed_close_to_the_surface() {
        assert_eq!(
            depth_strength(
                0.0,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
            0.0
        );
        assert_eq!(
            depth_strength(
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
            0.0
        );
        assert!(
            depth_strength(
                16.0,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ) > 0.0
        );
        assert_eq!(
            depth_strength(
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
            1.0
        );
    }

    #[test]
    fn cave_connectors_can_open_a_surface_mouth() {
        let strength = depth_strength(
            0.5,
            CAVE_CONNECTOR_MINIMUM_SURFACE_DEPTH,
            CAVE_CONNECTOR_FULL_STRENGTH_SURFACE_DEPTH,
        );
        let density = 0.5;
        let carved = density + carve_density_delta(density, strength, CAVE_CONNECTOR_AIR_MARGIN);

        assert!(strength > 0.0);
        assert!(carved < 0.0);
    }

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
