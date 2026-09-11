use bevy::prelude::*;

use crate::content::{
    biome::BiomeRegistry,
    biome_surface_carver::{BiomeSurfaceCarver, SurfaceCarverRange},
};

pub(super) fn surface_carver_density_delta(
    current_density: f32,
    position: Vec3,
    surface_height: i32,
    surface: &crate::world::biome_field::BiomeFieldSample<'_>,
    biomes: &BiomeRegistry,
    world_seed: u64,
) -> f32 {
    if current_density <= 0.0 {
        return 0.0;
    }

    let mut strongest = 0.0_f32;

    for influence in &surface.influences {
        let biome = biomes
            .get(influence.id)
            .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));

        for (index, carver) in biome.surface_carvers.iter().copied().enumerate() {
            let strength = tunnel_strength(
                position,
                surface_height,
                world_seed,
                biome.id.as_str(),
                index,
                carver,
            ) * influence.weight;
            strongest = strongest.max(strength);
        }
    }

    if strongest <= 0.0 {
        return 0.0;
    }

    -(current_density + 6.0) * strongest.clamp(0.0, 1.0)
}

fn tunnel_strength(
    position: Vec3,
    surface_height: i32,
    world_seed: u64,
    biome_id: &str,
    carver_index: usize,
    carver: BiomeSurfaceCarver,
) -> f32 {
    let BiomeSurfaceCarver::Tunnel {
        spacing,
        chance,
        length,
        radius,
        depth,
        jitter,
    } = carver;
    let horizontal = Vec2::new(position.x, position.z);
    let center = IVec2::new(
        (horizontal.x / spacing).floor() as i32,
        (horizontal.y / spacing).floor() as i32,
    );
    let maximum_reach = length.max * 0.5 + radius.max + jitter;
    let search_radius = (maximum_reach / spacing).ceil() as i32 + 1;
    let seed = mix_seed(
        world_seed
            ^ string_hash(biome_id)
            ^ (carver_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
    );
    let mut strongest = 0.0_f32;

    for z in -search_radius..=search_radius {
        for x in -search_radius..=search_radius {
            let cell = center + IVec2::new(x, z);
            let hash = cell_hash(cell, seed);
            if hash_unit(hash) > chance {
                continue;
            }

            let base = (cell.as_vec2() + Vec2::splat(0.5)) * spacing;
            let anchor = base
                + Vec2::new(
                    signed_unit(hash.rotate_left(13)) * jitter,
                    signed_unit(hash.rotate_left(31)) * jitter,
                );
            let angle = hash_unit(hash.rotate_left(47)) * std::f32::consts::TAU;
            let direction = Vec2::new(angle.cos(), angle.sin());
            let half_length = sample_range(length, hash.rotate_left(7)) * 0.5;
            let tunnel_radius = sample_range(radius, hash.rotate_left(23));
            let tunnel_depth = sample_range(depth, hash.rotate_left(41));
            let start = anchor - direction * half_length;
            let end = anchor + direction * half_length;
            let horizontal_distance = distance_to_segment(horizontal, start, end);

            if horizontal_distance >= tunnel_radius {
                continue;
            }

            let center_y = surface_height as f32 - tunnel_depth;
            let vertical_distance = (position.y - center_y).abs();
            let normalized = Vec2::new(
                horizontal_distance / tunnel_radius,
                vertical_distance / tunnel_radius,
            )
            .length();

            if normalized < 1.0 {
                strongest = strongest.max(smoothstep(1.0 - normalized));
            }
        }
    }

    strongest
}

fn distance_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_squared();
    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }

    let progress = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * progress)
}

fn sample_range(range: SurfaceCarverRange, hash: u64) -> f32 {
    range.min + (range.max - range.min) * hash_unit(hash)
}

fn cell_hash(cell: IVec2, seed: u64) -> u64 {
    let mut hash = seed;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    mix_seed(hash)
}

fn string_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

fn signed_unit(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
}

fn mix_seed(mut seed: u64) -> u64 {
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    seed ^= seed >> 33;
    seed
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}
