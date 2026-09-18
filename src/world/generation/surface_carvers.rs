use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_surface_carver::{BiomeSurfaceCarver, SurfaceCarverRange},
        dimension::DimensionDefinition,
    },
    world::{
        biome_field::BiomeField,
        feature_graph::FeatureGraph,
        terrain::surface_height,
    },
};

const MAXIMUM_TUNNEL_SLOPE: f32 = 0.06;
const TUNNEL_CURVE_STRENGTH: f32 = 0.38;
// Surface tunnels can span well over 100 blocks. Five samples left 30-40 block
// straight capsules visible in the terrain; keep the authored Bezier visibly curved.
const TUNNEL_PATH_SAMPLES: usize = 25;
const TUNNEL_MOUTH_BLEND_DEPTH: f32 = 12.0;
const TUNNEL_MOUTH_HORIZONTAL_FLARE: f32 = 0.85;
// Like lake shores, surface-tunnel mouths own a dry outer grading margin.
// Keep this in world blocks rather than a radius multiplier so small tunnels
// still receive a broad enough hillside transition.
const TUNNEL_MARGIN_OUTER_DISTANCE: f32 = 14.0;
const TUNNEL_MARGIN_SURFACE_OFFSET: f32 = 0.5;
const TUNNEL_MOUTH_CENTER_HEIGHT_FRACTION: f32 = 0.25;
const TUNNEL_CAVE_CONNECTION_MIN_ROOF_DEPTH: f32 = 6.0;
const TUNNEL_CONNECTION_SAMPLES_PER_SEGMENT: usize = 6;
const TUNNEL_MOUTH_SURFACE_OVERSHOOT: f32 = 0.5;

#[derive(Clone, Debug)]
struct ResolvedSurfaceTunnel {
    points: Vec<Vec3>,
    radius: f32,
    weight: f32,
}

#[derive(Debug, Default)]
pub(super) struct SurfaceCarverColumn {
    tunnels: Vec<ResolvedSurfaceTunnel>,
    margin_density_delta: f32,
}

pub(super) struct SurfaceCarverResolveContext<'a> {
    pub(super) dimension: &'a DimensionDefinition,
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
    surface_y: f32,
    surface_influences: &[(usize, f32)],
    context: &SurfaceCarverResolveContext<'_>,
) {
    column.tunnels.clear();
    column.margin_density_delta = 0.0;

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

    column.margin_density_delta =
        surface_tunnel_margin_density_delta(horizontal, surface_y, &column.tunnels);
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

    let carve_delta = if strongest > 0.0 {
        -(current_density + 6.0) * strongest.clamp(0.0, 1.0)
    } else {
        0.0
    };

    carve_delta + column.margin_density_delta
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
    let carver_maximum = sea_level
        + elevation.max
        + radius.max
        + maximum_vertical_half_span
        + TUNNEL_MOUTH_BLEND_DEPTH;

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
    let maximum_mouth_radius =
        radius.max * (1.0 + TUNNEL_MOUTH_HORIZONTAL_FLARE);
    let maximum_reach =
        length.max * 0.5 + maximum_mouth_radius + TUNNEL_MARGIN_OUTER_DISTANCE + jitter;
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
            let points = (0..TUNNEL_PATH_SAMPLES)
                .map(|index| {
                    let t = index as f32 / (TUNNEL_PATH_SAMPLES - 1) as f32;
                    quadratic_bezier(start, control, end, t)
                })
                .collect::<Vec<_>>();
            let Some(points) = connected_surface_tunnel(points, tunnel_radius, context) else {
                continue;
            };

            tunnels.push(ResolvedSurfaceTunnel {
                points,
                radius: tunnel_radius,
                weight,
            });
        }
    }
}

fn connected_surface_tunnel(
    mut points: Vec<Vec3>,
    radius: f32,
    context: &SurfaceCarverResolveContext<'_>,
) -> Option<Vec<Vec3>> {
    let cave_graph = context.cave_graph?;
    let mut surface_at = |horizontal: Vec2| {
        let block_position =
            IVec2::new(horizontal.x.floor() as i32, horizontal.y.floor() as i32);
        surface_height(
            block_position,
            context.dimension,
            context.biomes,
            context.biome_field,
        ) as f32
    };

    let (mouth_index, mouth_surface) = points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            let horizontal = Vec2::new(point.x, point.z);
            let surface = surface_at(horizontal);
            let roof_gap = surface - (point.y + radius);
            (roof_gap.abs() <= TUNNEL_MOUTH_BLEND_DEPTH)
                .then_some((index, surface, roof_gap.abs()))
        })
        .min_by(|left, right| {
            left.2
                .total_cmp(&right.2)
                .then_with(|| left.0.cmp(&right.0))
        })
        .map(|(index, surface, _)| (index, surface))?;

    points[mouth_index].y = mouth_surface + TUNNEL_MOUTH_SURFACE_OVERSHOOT - radius;

    let forward = tunnel_branch_to_cave(
        &points,
        mouth_index,
        1,
        radius,
        cave_graph,
        &mut surface_at,
    );
    let backward = tunnel_branch_to_cave(
        &points,
        mouth_index,
        -1,
        radius,
        cave_graph,
        &mut surface_at,
    );

    match (forward, backward) {
        (Some(left), Some(right)) => {
            if polyline_length(&left) <= polyline_length(&right) {
                Some(left)
            } else {
                Some(right)
            }
        }
        (Some(path), None) | (None, Some(path)) => Some(path),
        (None, None) => None,
    }
}

fn tunnel_branch_to_cave(
    points: &[Vec3],
    mouth_index: usize,
    direction: i32,
    radius: f32,
    cave_graph: &FeatureGraph,
    surface_at: &mut impl FnMut(Vec2) -> f32,
) -> Option<Vec<Vec3>> {
    let mut branch = vec![points[mouth_index]];
    let mut index = mouth_index as i32;

    loop {
        let next = index + direction;
        if next < 0 || next >= points.len() as i32 {
            return None;
        }

        let start = *branch
            .last()
            .expect("surface tunnel branch must contain its mouth");
        let end = points[next as usize];

        for sample_index in 1..=TUNNEL_CONNECTION_SAMPLES_PER_SEGMENT {
            let t = sample_index as f32 / TUNNEL_CONNECTION_SAMPLES_PER_SEGMENT as f32;
            let position = start.lerp(end, t);
            let horizontal = Vec2::new(position.x, position.z);
            let roof_depth = surface_at(horizontal) - (position.y + radius);
            if roof_depth >= TUNNEL_CAVE_CONNECTION_MIN_ROOF_DEPTH
                && cave_graph.sample_with_margin(position, radius).is_some()
            {
                branch.push(position);
                return Some(branch);
            }
        }

        branch.push(end);
        index = next;
    }
}

fn polyline_length(points: &[Vec3]) -> f32 {
    points
        .windows(2)
        .map(|segment| segment[0].distance(segment[1]))
        .sum()
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

fn surface_tunnel_margin_density_delta(
    horizontal: Vec2,
    surface_y: f32,
    tunnels: &[ResolvedSurfaceTunnel],
) -> f32 {
    let mut strongest_delta = 0.0_f32;

    for tunnel in tunnels {
        let Some((horizontal_distance, path_y)) = tunnel
            .points
            .windows(2)
            .map(|segment| horizontal_distance_to_segment(horizontal, segment[0], segment[1]))
            .min_by(|left, right| left.0.total_cmp(&right.0))
        else {
            continue;
        };

        let tunnel_top = path_y + tunnel.radius;
        let depth_to_tunnel = (surface_y - tunnel_top).max(0.0);
        if depth_to_tunnel >= TUNNEL_MOUTH_BLEND_DEPTH {
            continue;
        }

        let exposure = smoothstep(
            (1.0 - depth_to_tunnel / TUNNEL_MOUTH_BLEND_DEPTH).clamp(0.0, 1.0),
        );
        let core_radius =
            tunnel.radius * (1.0 + TUNNEL_MOUTH_HORIZONTAL_FLARE * exposure);
        let normalized_distance = horizontal_distance / core_radius.max(f32::EPSILON);
        let margin_strength = tunnel_margin_strength(horizontal_distance, core_radius);
        if margin_strength <= 0.0 {
            continue;
        }

        // Lake-style basin semantics for the tunnel mouth:
        // - center of the core lowers the terrain toward the tunnel interior;
        // - core boundary converges to the tunnel roof;
        // - outer dry margin fades from that roof back to natural terrain.
        let core_opening =
            smoothstep(1.0 - normalized_distance.clamp(0.0, 1.0));
        let roof_surface = tunnel_top + TUNNEL_MARGIN_SURFACE_OFFSET;
        let center_surface =
            path_y + tunnel.radius * TUNNEL_MOUTH_CENTER_HEIGHT_FRACTION;
        let target_surface = if normalized_distance <= 1.0 {
            lerp(roof_surface, center_surface, core_opening)
        } else {
            roof_surface
        };
        let height_delta = (target_surface - surface_y).min(0.0);
        let delta = height_delta * margin_strength * exposure * tunnel.weight;

        if delta.abs() > strongest_delta.abs() {
            strongest_delta = delta;
        }
    }

    strongest_delta
}

fn horizontal_distance_to_segment(point: Vec2, start: Vec3, end: Vec3) -> (f32, f32) {
    let start_horizontal = Vec2::new(start.x, start.z);
    let end_horizontal = Vec2::new(end.x, end.z);
    let segment = end_horizontal - start_horizontal;
    let length_squared = segment.length_squared();
    let progress = if length_squared <= f32::EPSILON {
        0.0
    } else {
        ((point - start_horizontal).dot(segment) / length_squared).clamp(0.0, 1.0)
    };
    let closest = start_horizontal + segment * progress;
    let path_y = start.y + (end.y - start.y) * progress;

    (point.distance(closest), path_y)
}

fn tunnel_margin_strength(horizontal_distance: f32, core_radius: f32) -> f32 {
    if horizontal_distance <= core_radius {
        return 1.0;
    }
    let outer_radius = core_radius + TUNNEL_MARGIN_OUTER_DISTANCE;
    if horizontal_distance >= outer_radius {
        return 0.0;
    }

    let progress =
        1.0 - (horizontal_distance - core_radius) / TUNNEL_MARGIN_OUTER_DISTANCE;
    smoothstep(progress.clamp(0.0, 1.0))
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

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
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
    fn branch_requires_a_deep_cave_intersection() {
        let points = vec![
            Vec3::new(0.0, 60.0, 0.0),
            Vec3::new(10.0, 56.0, 0.0),
            Vec3::new(20.0, 50.0, 0.0),
        ];
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::new(18.0, 50.0, -10.0));
        let to = graph.add_node(Vec3::new(18.0, 50.0, 10.0));
        graph.add_edge(from, to, 4.0, 4.0);
        let mut surface_at = |_horizontal: Vec2| 64.0;

        let branch = tunnel_branch_to_cave(
            &points,
            0,
            1,
            4.0,
            &graph,
            &mut surface_at,
        )
        .expect("deep intersection should connect the branch");

        assert!(branch.len() >= 2);
        assert!(branch.last().unwrap().x < points.last().unwrap().x);
    }

    #[test]
    fn branch_rejects_surface_only_graph_touch() {
        let points = vec![
            Vec3::new(0.0, 60.0, 0.0),
            Vec3::new(10.0, 60.0, 0.0),
            Vec3::new(20.0, 60.0, 0.0),
        ];
        let mut graph = FeatureGraph::default();
        let from = graph.add_node(Vec3::new(10.0, 60.0, -10.0));
        let to = graph.add_node(Vec3::new(10.0, 60.0, 10.0));
        graph.add_edge(from, to, 5.0, 5.0);
        let mut surface_at = |_horizontal: Vec2| 64.0;

        assert!(
            tunnel_branch_to_cave(
                &points,
                0,
                1,
                4.0,
                &graph,
                &mut surface_at,
            )
            .is_none()
        );
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

    #[test]
    fn tunnel_margin_matches_lake_style_falloff() {
        let core_radius = 10.0;
        assert_eq!(tunnel_margin_strength(core_radius, core_radius), 1.0);
        assert!(tunnel_margin_strength(core_radius + 2.0, core_radius) > 0.0);
        assert!(
            tunnel_margin_strength(core_radius + 2.0, core_radius)
                > tunnel_margin_strength(
                    core_radius + TUNNEL_MARGIN_OUTER_DISTANCE - 2.0,
                    core_radius,
                )
        );
        assert_eq!(
            tunnel_margin_strength(
                core_radius + TUNNEL_MARGIN_OUTER_DISTANCE,
                core_radius,
            ),
            0.0
        );
    }

    #[test]
    fn surface_tunnel_margin_grades_high_terrain_outside_core() {
        let tunnel = ResolvedSurfaceTunnel {
            points: (0..TUNNEL_PATH_SAMPLES)
                .map(|index| {
                    let x = -12.0 + index as f32 * 2.0;
                    Vec3::new(x, 50.0, 0.0)
                })
                .collect(),
            radius: 6.0,
            weight: 1.0,
        };
        let core_radius = 6.0 * (1.0 + TUNNEL_MOUTH_HORIZONTAL_FLARE);
        let horizontal = Vec2::new(0.0, core_radius + 4.0);
        let delta = surface_tunnel_margin_density_delta(horizontal, 58.0, &[tunnel]);

        assert!(delta < 0.0);
    }

    #[test]
    fn deep_tunnel_has_no_surface_margin() {
        let tunnel = ResolvedSurfaceTunnel {
            points: (0..TUNNEL_PATH_SAMPLES)
                .map(|index| {
                    let x = -12.0 + index as f32 * 1.0;
                    Vec3::new(x, 30.0, 0.0)
                })
                .collect(),
            radius: 6.0,
            weight: 1.0,
        };

        assert_eq!(
            surface_tunnel_margin_density_delta(Vec2::new(0.0, 8.0), 60.0, &[tunnel]),
            0.0
        );
    }

    #[test]
    fn tunnel_core_surface_is_graded_before_3d_carve() {
        let tunnel = ResolvedSurfaceTunnel {
            points: (0..TUNNEL_PATH_SAMPLES)
                .map(|index| {
                    let x = -12.0 + index as f32 * 1.0;
                    Vec3::new(x, 50.0, 0.0)
                })
                .collect(),
            radius: 6.0,
            weight: 1.0,
        };

        let center = surface_tunnel_margin_density_delta(Vec2::ZERO, 58.0, &[tunnel]);
        let boundary = surface_tunnel_margin_density_delta(Vec2::new(0.0, 10.0), 58.0, &[tunnel]);

        assert!(center < 0.0);
        assert!(center <= boundary);
    }
}
