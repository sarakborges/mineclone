use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_surface_carver::{BiomeSurfaceCarver, SurfaceCarverRange},
    },
    world::biome_field::BiomeField,
};

const MAXIMUM_TUNNEL_SLOPE: f32 = 0.06;
const TUNNEL_CURVE_STRENGTH: f32 = 0.38;
const TUNNEL_PATH_SAMPLES: usize = 5;

#[derive(Clone, Copy, Debug)]
struct ResolvedSurfaceTunnel {
    points: [Vec3; TUNNEL_PATH_SAMPLES],
    radius: f32,
    weight: f32,
}

#[derive(Debug, Default)]
pub(super) struct SurfaceCarverColumn {
    tunnels: Vec<ResolvedSurfaceTunnel>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_surface_carver_column(
    horizontal: Vec2,
    surface_influences: &[(usize, f32)],
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    world_seed: u64,
    sea_level: f32,
    minimum_y: f32,
    maximum_y: f32,
) -> SurfaceCarverColumn {
    let mut tunnels = Vec::new();

    for &(biome_index, weight) in surface_influences {
        if weight <= 0.0 {
            continue;
        }

        let biome_id = biome_field.surface_biome_id(biome_index);
        let biome = biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

        for (index, carver) in biome.surface_carvers.iter().copied().enumerate() {
            if !carver_intersects_vertical_range(carver, sea_level, minimum_y, maximum_y) {
                continue;
            }

            resolve_tunnel_candidates(
                &mut tunnels,
                horizontal,
                world_seed,
                sea_level,
                biome.id.as_str(),
                index,
                carver,
                weight,
            );
        }
    }

    SurfaceCarverColumn { tunnels }
}

pub(super) fn surface_carver_density_delta(
    current_density: f32,
    position: Vec3,
    column: &SurfaceCarverColumn,
) -> f32 {
    if current_density <= 0.0 || column.tunnels.is_empty() {
        return 0.0;
    }

    let mut strongest = 0.0_f32;

    for tunnel in &column.tunnels {
        let distance = tunnel
            .points
            .windows(2)
            .map(|segment| distance_to_segment(position, segment[0], segment[1]))
            .min_by(f32::total_cmp)
            .unwrap_or(f32::MAX);

        if distance < tunnel.radius {
            let strength = smoothstep(1.0 - distance / tunnel.radius) * tunnel.weight;
            strongest = strongest.max(strength);
        }
    }

    if strongest <= 0.0 {
        return 0.0;
    }

    -(current_density + 6.0) * strongest.clamp(0.0, 1.0)
}

fn carver_intersects_vertical_range(
    carver: BiomeSurfaceCarver,
    sea_level: f32,
    minimum_y: f32,
    maximum_y: f32,
) -> bool {
    let BiomeSurfaceCarver::Tunnel {
        length,
        radius,
        elevation,
        ..
    } = carver;
    let maximum_vertical_half_span = length.max * 0.5 * MAXIMUM_TUNNEL_SLOPE;
    let carver_minimum =
        sea_level + elevation.min - radius.max - maximum_vertical_half_span;
    let carver_maximum =
        sea_level + elevation.max + radius.max + maximum_vertical_half_span;

    carver_maximum >= minimum_y && carver_minimum <= maximum_y
}

#[allow(clippy::too_many_arguments)]
fn resolve_tunnel_candidates(
    tunnels: &mut Vec<ResolvedSurfaceTunnel>,
    horizontal: Vec2,
    world_seed: u64,
    sea_level: f32,
    biome_id: &str,
    carver_index: usize,
    carver: BiomeSurfaceCarver,
    weight: f32,
) {
    let BiomeSurfaceCarver::Tunnel {
        spacing,
        chance,
        length,
        radius,
        elevation,
        jitter,
    } = carver;
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
            let perpendicular = Vec2::new(-direction.y, direction.x);
            let half_length = sample_range(length, hash.rotate_left(7)) * 0.5;
            let tunnel_radius = sample_range(radius, hash.rotate_left(23));
            let center_y = sea_level + sample_range(elevation, hash.rotate_left(41));
            let vertical_half_span =
                signed_unit(hash.rotate_left(59)) * half_length * MAXIMUM_TUNNEL_SLOPE;
            let start_horizontal = anchor - direction * half_length;
            let end_horizontal = anchor + direction * half_length;
            let curve_offset = signed_unit(hash.rotate_left(17))
                * half_length
                * TUNNEL_CURVE_STRENGTH;
            let control_horizontal = anchor + perpendicular * curve_offset;
            let control_y = center_y + signed_unit(hash.rotate_left(37)) * tunnel_radius * 0.8;
            let start = Vec3::new(
                start_horizontal.x,
                center_y - vertical_half_span,
                start_horizontal.y,
            );
            let control = Vec3::new(control_horizontal.x, control_y, control_horizontal.y);
            let end = Vec3::new(
                end_horizontal.x,
                center_y + vertical_half_span,
                end_horizontal.y,
            );
            let points = std::array::from_fn(|index| {
                let t = index as f32 / (TUNNEL_PATH_SAMPLES - 1) as f32;
                quadratic_bezier(start, control, end, t)
            });

            tunnels.push(ResolvedSurfaceTunnel {
                points,
                radius: tunnel_radius,
                weight,
            });
        }
    }
}

fn quadratic_bezier(start: Vec3, control: Vec3, end: Vec3, t: f32) -> Vec3 {
    let inverse = 1.0 - t;
    start * inverse * inverse + control * (2.0 * inverse * t) + end * t * t
}

fn distance_to_segment(point: Vec3, start: Vec3, end: Vec3) -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunnel_range_rejects_chunks_that_cannot_intersect() {
        let carver = BiomeSurfaceCarver::Tunnel {
            spacing: 100.0,
            chance: 1.0,
            length: SurfaceCarverRange { min: 80.0, max: 120.0 },
            radius: SurfaceCarverRange { min: 6.0, max: 10.0 },
            elevation: SurfaceCarverRange { min: 10.0, max: 30.0 },
            jitter: 20.0,
        };

        assert!(!carver_intersects_vertical_range(carver, 64.0, 0.0, 31.0));
        assert!(carver_intersects_vertical_range(carver, 64.0, 64.0, 111.0));
    }

    #[test]
    fn quadratic_path_passes_through_both_endpoints() {
        let start = Vec3::new(-10.0, 2.0, 0.0);
        let control = Vec3::new(0.0, 5.0, 8.0);
        let end = Vec3::new(10.0, 3.0, 0.0);

        assert_eq!(quadratic_bezier(start, control, end, 0.0), start);
        assert_eq!(quadratic_bezier(start, control, end, 1.0), end);
        assert_ne!(quadratic_bezier(start, control, end, 0.5).z, 0.0);
    }
}
