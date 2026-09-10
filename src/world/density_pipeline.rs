use bevy::prelude::*;

use crate::content::biome_density::BiomeDensityModifier;

use super::{
    biome_field::{BiomeField, VolumeBiomeSelection},
    cave_connectivity::CaveConnectivityRegion,
    generation_region::GenerationRegion,
};

const DENSITY_NOISE_EDGE: f32 = 0.15;
const CAVE_CONNECTOR_AIR_MARGIN: f32 = 4.0;

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

    if let Some(connector) = anchored_caves
        .and_then(|caves| caves.connector_graph.sample(position))
        .map(|sample| smoothstep(sample.strength))
    {
        density += carve_density_delta(density, connector, CAVE_CONNECTOR_AIR_MARGIN);
    }

    density + volume_biome_density_delta(density, position, volume, biome_field)
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
) -> f32 {
    let Some(selection) = volume else {
        return 0.0;
    };
    let Some((modifier, seed)) = biome_field.volume_density_modifier(selection) else {
        return 0.0;
    };

    density_modifier_delta(modifier, current_density, position, seed) * selection.strength
}

fn density_modifier_delta(
    modifier: BiomeDensityModifier,
    current_density: f32,
    position: Vec3,
    seed: u64,
) -> f32 {
    match modifier {
        BiomeDensityModifier::Cavern {
            carve_strength,
            noise_scale,
            openness,
        } => {
            let noise = value_noise_3d(position * noise_scale, seed) * 0.5 + 0.5;
            let mask = coverage_mask(noise, openness);
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
        );

        assert!(current_density + cavern < 0.0);
        assert_eq!(solid, 20.0);
    }
}
