use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_surface_carver::{BiomeSurfaceCarver, SurfaceCarverRange},
    },
    world::{biome_field::BiomeField, feature_graph::FeatureGraph},
};

const MAXIMUM_TUNNEL_SLOPE: f32 = 0.06;
const TUNNEL_CURVE_STRENGTH: f32 = 0.38;
// Surface tunnels can span well over 100 blocks. Five samples left 30-40 block
// straight capsules visible in the terrain; keep the authored Bezier visibly curved.
const TUNNEL_PATH_SAMPLES: usize = 13;
const TUNNEL_MOUTH_BLEND_DEPTH: f32 = 12.0;
const TUNNEL_MOUTH_HORIZONTAL_FLARE: f32 = 0.85;

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

pub(super) struct SurfaceCarverResolveContext<'a> {
    pub(super) biomes: &'a BiomeRegistry,
    pub(super) biome_field: &'a BiomeField,
    pub(super) world_seed: u64,
    pub(super) sea_level: f32,
    pub(super) minimum_y: f32,
    pub(super) maximum_y: f32,
    pub(super) cave_graph: Option<&'a FeatureGraph>,
}

struct SurfaceTunnelCandidate<'a> {
    biome_id: &'a str,
    carver_index: usize,
    carver: BiomeSurfaceCarver,
    weight: f32,
}

pub(super) fn resolve_surface_carver_column(
    column: &mut SurfaceCarverColumn,
    horizontal: Vec2,
    surface_influences: &[(usize, f32)],
    context: &SurfaceCarverResolveContext<'_>,
) {
    column.tunnels.clear();

    for &(biome_index, weight) in surface_influences {
        if weight <= 0.0 {
            continue;
        }

        let biome_id = context.biome_field.surface_biome_id(biome_index);
        let biome = context
            .biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

        for (index, carver) in biome.surface_carvers.iter().copied().enumerate() {
            if !carver_intersects_vertical_range(
                carver,
                context.sea_level,
                context.minimum_y,
                context.maximum_y,
            ) {
                continue;
            }

            resolve_tunnel_candidates(
                &mut column.tunnels,
                horizontal,
                context,
                SurfaceTunnelCandidate {
                    biome_id: biome.id.as_str(),
                    carver_index: index,
                    carver,
                    weight,
                },
            );
        }
    }
}

pub(super) fn surface_carver_density_delta(
    current_density: f32,
    position: Vec3,
    surface_y: f32,
    column: &SurfaceCarverColumn,
) -> f32 {
    if current_density <= 0.0 || column.tunnels.is_empty() {
        return 0.0;
    }

    let mut strongest = 0.0_f32;

    for tunnel in &column.tunnels {
        let normalized_distance = tunnel
            .points
            .windows(2)
            .map(|segment| {
                tunnel_normalized_distance(
                    position,
                    segment[0],
                    segment[1],
                    tunnel.radius,
                    surface_y,
                )
            })
            .min_by(f32::total_cmp)
            .unwrap_or(f32::MAX);

        if normalized_distance < 1.0 {
            let strength = smoothstep(1.0 - normalized_distance) * tunnel.weight;
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
    let carver_minimum = sea_level + elevation.min - radius.max - maximum_vertical_half_span;
    let carver_maximum = sea_level + elevation.max + radius.max + maximum_vertical_half_span;

    carver_maximum >= minimum_y && carver_minimum <= maximum_y
}

fn resolve_tunnel_candidates(
    tunnels: &mut Vec<ResolvedSurfaceTunnel>,
    horizontal: Vec2,
    context: &SurfaceCarverResolveContext<'_>,
    candidate: SurfaceTunnelCandidate<'_>,
) {
    let SurfaceTunnelCandidate {
        biome_id,
        carver_index,
        carver,
        weight,
    } = candidate;
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
        context.world_seed
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
            let center_y = context.sea_level + sample_range(elevation, hash.rotate_left(41));
            let vertical_half_span =
                signed_unit(hash.rotate_left(59)) * half_length * MAXIMUM_TUNNEL_SLOPE;
            let start_horizontal = anchor - direction * half_length;
            let end_horizontal = anchor + direction * half_length;
            let curve_offset =
                signed_unit(hash.rotate_left(17)) * half_length * TUNNEL_CURVE_STRENGTH;
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

            if !tunnel_connects_to_cave(&points, tunnel_radius, context.cave_graph) {
                continue;
            }

            tunnels.push(ResolvedSurfaceTunnel {
                points,
                radius: tunnel_radius,
                weight,
            });
        }
    }
}

fn tunnel_connects_to_cave(
    points: &[Vec3],
    radius: f32,
    cave_graph: Option<&FeatureGraph>,
) -> bool {
    let Some(cave_graph) = cave_graph else {
        return false;
    };

    const INTERSECTION_SAMPLES_PER_SEGMENT: usize = 4;
    points.windows(2).any(|segment| {
        (0..=INTERSECTION_SAMPLES_PER_SEGMENT).any(|index| {
            let t = index as f32 / INTERSECTION_SAMPLES_PER_SEGMENT as f32;
            let position = segment[0].lerp(segment[1], t);
            cave_graph.sample_with_margin(position, radius).is_some()
        })
    })
}

fn quadratic_bezier(start: Vec3, control: Vec3, end: Vec3, t: f32) -> Vec3 {
    let inverse = 1.0 - t;
    start * inverse * inverse + control * (2.0 * inverse * t) + end * t * t
}

fn tunnel_normalized_distance(
    point: Vec3,
    start: Vec3,
    end: Vec3,
    radius: f32,
    surface_y: f32,
) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_squared();
    let closest = if length_squared <= f32::EPSILON {
        start
    } else {
        let progress = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
        start + segment * progress
    };
    let delta = point - closest;

    // Underground the profile remains circular. In the last few blocks below
    // the terrain surface, widen only the horizontal axes. That turns an
    // exposed tunnel mouth into a gradual cut through the hillside instead of
    // carrying the cylinder's near-vertical side wall all the way to the top.
    let depth_below_surface = (surface_y - point.y).max(0.0);
    let mouth_progress =
        (1.0 - depth_below_surface / TUNNEL_MOUTH_BLEND_DEPTH).clamp(0.0, 1.0);
    let mouth_strength = smoothstep(mouth_progress);
    let horizontal_radius =
        radius * (1.0 + TUNNEL_MOUTH_HORIZONTAL_FLARE * mouth_strength);

    let horizontal = Vec2::new(delta.x, delta.z).length() / horizontal_radius;
    let vertical = delta.y / radius;

    (horizontal * horizontal + vertical * vertical).sqrt()
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
            length: SurfaceCarverRange {
                min: 80.0,
                max: 120.0,
            },
            radius: SurfaceCarverRange {
                min: 6.0,
                max: 10.0,
            },
            elevation: SurfaceCarverRange {
                min: 10.0,
                max: 30.0,
            },
            jitter: 20.0,
        };

        assert!(!carver_intersects_vertical_range(carver, 64.0, 0.0, 31.0));
        assert!(carver_intersects_vertical_range(carver, 64.0, 64.0, 111.0));
    }

    #[test]
    fn disconnected_surface_tunnel_is_rejected() {
        let points = [
            Vec3::new(0.0, 64.0, 0.0),
            Vec3::new(5.0, 64.0, 0.0),
            Vec3::new(10.0, 64.0, 0.0),
            Vec3::new(15.0, 64.0, 0.0),
            Vec3::new(20.0, 64.0, 0.0),
        ];
        assert!(!tunnel_connects_to_cave(&points, 3.0, None));

        let graph = FeatureGraph::default();
        assert!(!tunnel_connects_to_cave(&points, 3.0, Some(&graph)));
    }

    #[test]
    fn surface_tunnel_touching_cave_graph_is_kept() {
        let points = [
            Vec3::new(0.0, 64.0, 0.0),
            Vec3::new(5.0, 64.0, 0.0),
            Vec3::new(10.0, 64.0, 0.0),
            Vec3::new(15.0, 64.0, 0.0),
            Vec3::new(20.0, 64.0, 0.0),
        ];
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::new(10.0, 56.0, -10.0));
        let to = graph.add_node(Vec3::new(10.0, 56.0, 10.0));
        graph.add_edge(from, to, 5.0, 5.0);

        assert!(tunnel_connects_to_cave(&points, 4.0, Some(&graph)));
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

    #[test]
    fn tunnel_mouth_flares_horizontally_near_surface() {
        let start = Vec3::new(-10.0, 50.0, 0.0);
        let end = Vec3::new(10.0, 50.0, 0.0);
        let radius = 6.0;
        let offset = 7.0;

        let deep = tunnel_normalized_distance(
            Vec3::new(0.0, 50.0, offset),
            start,
            end,
            radius,
            70.0,
        );
        let near_surface = tunnel_normalized_distance(
            Vec3::new(0.0, 50.0, offset),
            start,
            end,
            radius,
            52.0,
        );

        assert!(deep > 1.0);
        assert!(near_surface < 1.0);
    }

    #[test]
    fn deep_tunnel_profile_remains_circular() {
        let start = Vec3::new(-10.0, 30.0, 0.0);
        let end = Vec3::new(10.0, 30.0, 0.0);
        let radius = 6.0;

        let horizontal = tunnel_normalized_distance(
            Vec3::new(0.0, 30.0, 3.0),
            start,
            end,
            radius,
            60.0,
        );
        let vertical = tunnel_normalized_distance(
            Vec3::new(0.0, 33.0, 0.0),
            start,
            end,
            radius,
            60.0,
        );

        assert!((horizontal - 0.5).abs() <= f32::EPSILON);
        assert!((vertical - 0.5).abs() <= f32::EPSILON);
    }
}
