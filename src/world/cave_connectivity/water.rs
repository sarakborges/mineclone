use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

const UNDERGROUND_LAKE_CHANCE: f32 = 0.38;
const UNDERGROUND_LAKE_MINIMUM_RADIUS: f32 = 8.0;
const UNDERGROUND_LAKE_MAXIMUM_RADIUS: f32 = 22.0;
const UNDERGROUND_LAKE_MINIMUM_DEPTH: f32 = 3.0;
const UNDERGROUND_LAKE_MAXIMUM_DEPTH: f32 = 7.5;
const UNDERGROUND_LAKE_SURFACE_OFFSET: f32 = 1.25;
const UNDERGROUND_RIVER_CHANCE: f32 = 0.28;
const UNDERGROUND_LAKE_CONNECTION_CHANCE: f32 = 0.86;
const UNDERGROUND_RIVER_RADIUS_SCALE: f32 = 0.48;
const UNDERGROUND_RIVER_MINIMUM_RADIUS: f32 = 1.25;
const UNDERGROUND_RIVER_MAXIMUM_RADIUS: f32 = 4.0;
const UNDERGROUND_RIVER_SURFACE_RADIUS_OFFSET: f32 = 0.38;
const UNDERGROUND_RIVER_MINIMUM_SURFACE_OFFSET: f32 = 1.0;
const UNDERGROUND_RIVER_MINIMUM_DEPTH: f32 = 0.75;
const UNDERGROUND_RIVER_MAXIMUM_DEPTH: f32 = 2.75;
const UNDERGROUND_WATERFALL_MINIMUM_DROP: f32 = 4.0;
const UNDERGROUND_WATERFALL_MINIMUM_SLOPE: f32 = 0.42;
const UNDERGROUND_WATERFALL_RADIUS_SCALE: f32 = 0.62;
const UNDERGROUND_WATERFALL_MINIMUM_RADIUS: f32 = 1.0;
const UNDERGROUND_WATERFALL_MAXIMUM_RADIUS: f32 = 3.25;
const MINIMUM_WATER_Y: f32 = 2.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct UndergroundWaterSample {
    pub water_level: f32,
    pub bed_level: f32,
}

#[derive(Clone, Debug)]
struct UndergroundLake {
    center: Vec2,
    radius: Vec2,
    rotation: f32,
    shape_seed: u64,
    water_level: f32,
    depth: f32,
}

impl UndergroundLake {
    fn horizontal_strength(&self, position: Vec2) -> f32 {
        let delta = position - self.center;
        let (sin, cos) = self.rotation.sin_cos();
        let local = Vec2::new(
            delta.x * cos + delta.y * sin,
            -delta.x * sin + delta.y * cos,
        );
        let normalized = Vec2::new(local.x / self.radius.x, local.y / self.radius.y);
        let boundary_scale = irregular_boundary_scale(normalized, self.shape_seed);
        let distance = normalized.length() / boundary_scale;

        smoothstep(1.0 - distance.clamp(0.0, 1.0))
    }
}

#[derive(Clone, Copy, Debug)]
struct UndergroundWaterfall {
    from: Vec3,
    to: Vec3,
    radius: f32,
}

impl UndergroundWaterfall {
    fn contains(&self, position: Vec3) -> bool {
        let segment = self.to - self.from;
        let length_squared = segment.length_squared();
        if length_squared <= f32::EPSILON {
            return false;
        }

        let progress = ((position - self.from).dot(segment) / length_squared).clamp(0.0, 1.0);
        let closest = self.from + segment * progress;
        position.distance(closest) <= self.radius
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct UndergroundWaterRegion {
    river_graph: FeatureGraph,
    lakes: Vec<UndergroundLake>,
    waterfalls: Vec<UndergroundWaterfall>,
}

impl UndergroundWaterRegion {
    pub(super) fn from_anchors(anchors: &[Vec3], seed: u64) -> Self {
        let mut anchors = anchors.to_vec();
        anchors.sort_by(compare_position);
        anchors.dedup_by(|left, right| *left == *right);

        let lakes = anchors
            .into_iter()
            .filter_map(|anchor| lake_for_anchor(anchor, seed))
            .collect();

        Self {
            river_graph: FeatureGraph::default(),
            lakes,
            waterfalls: Vec::new(),
        }
    }

    pub(super) fn connection_carries_water(from: Vec3, to: Vec3, seed: u64) -> bool {
        let from_lake = anchor_has_lake(from, seed);
        let to_lake = anchor_has_lake(to, seed);
        let high_lake_drains_down = (from_lake && from.y > to.y + 2.0)
            || (to_lake && to.y > from.y + 2.0);
        if high_lake_drains_down {
            return true;
        }

        let hash = pair_hash(from, to, seed ^ 0x510e_527f_ade6_82d1);
        let chance = if from_lake || to_lake {
            UNDERGROUND_LAKE_CONNECTION_CHANCE
        } else {
            UNDERGROUND_RIVER_CHANCE
        };

        hash_unit(hash.rotate_left(17)) < chance
    }

    pub(super) fn add_river_segment(
        &mut self,
        from: Vec3,
        to: Vec3,
        start_cave_radius: f32,
        end_cave_radius: f32,
    ) {
        let start_surface_offset =
            (start_cave_radius * UNDERGROUND_RIVER_SURFACE_RADIUS_OFFSET)
                .max(UNDERGROUND_RIVER_MINIMUM_SURFACE_OFFSET);
        let end_surface_offset = (end_cave_radius * UNDERGROUND_RIVER_SURFACE_RADIUS_OFFSET)
            .max(UNDERGROUND_RIVER_MINIMUM_SURFACE_OFFSET);
        let river_from = from - Vec3::Y * start_surface_offset;
        let river_to = to - Vec3::Y * end_surface_offset;
        let start_radius = (start_cave_radius * UNDERGROUND_RIVER_RADIUS_SCALE)
            .clamp(UNDERGROUND_RIVER_MINIMUM_RADIUS, UNDERGROUND_RIVER_MAXIMUM_RADIUS);
        let end_radius = (end_cave_radius * UNDERGROUND_RIVER_RADIUS_SCALE)
            .clamp(UNDERGROUND_RIVER_MINIMUM_RADIUS, UNDERGROUND_RIVER_MAXIMUM_RADIUS);
        let from_node = self.river_graph.add_node(river_from);
        let to_node = self.river_graph.add_node(river_to);

        self.river_graph
            .add_edge(from_node, to_node, start_radius, end_radius);

        let vertical_drop = (river_from.y - river_to.y).abs();
        let horizontal_distance = Vec2::new(
            river_to.x - river_from.x,
            river_to.z - river_from.z,
        )
        .length()
        .max(0.5);
        let slope = vertical_drop / horizontal_distance;
        if vertical_drop >= UNDERGROUND_WATERFALL_MINIMUM_DROP
            && slope >= UNDERGROUND_WATERFALL_MINIMUM_SLOPE
        {
            let radius = (start_radius.max(end_radius) * UNDERGROUND_WATERFALL_RADIUS_SCALE)
                .clamp(
                    UNDERGROUND_WATERFALL_MINIMUM_RADIUS,
                    UNDERGROUND_WATERFALL_MAXIMUM_RADIUS,
                );
            let (high, low) = if river_from.y >= river_to.y {
                (river_from, river_to)
            } else {
                (river_to, river_from)
            };
            self.waterfalls.push(UndergroundWaterfall {
                from: high,
                to: low,
                radius,
            });
        }
    }

    pub(crate) fn water_at(&self, position: Vec3) -> Option<UndergroundWaterSample> {
        for waterfall in &self.waterfalls {
            if waterfall.contains(position) {
                return Some(UndergroundWaterSample {
                    water_level: position.y + 0.5,
                    bed_level: position.y - 0.5,
                });
            }
        }

        let horizontal = Vec2::new(position.x, position.z);
        let mut selected = None;

        for lake in &self.lakes {
            let strength = lake.horizontal_strength(horizontal);
            if strength <= 0.0 {
                continue;
            }

            consider_water(
                &mut selected,
                UndergroundWaterSample {
                    water_level: lake.water_level,
                    bed_level: lake.water_level - lake.depth * strength,
                },
                position.y,
            );
        }

        if let Some(river) = self.river_graph.sample_horizontal(horizontal) {
            let strength = smoothstep(river.strength.clamp(0.0, 1.0));
            consider_water(
                &mut selected,
                UndergroundWaterSample {
                    water_level: river.height,
                    bed_level: river.height
                        - lerp(
                            UNDERGROUND_RIVER_MINIMUM_DEPTH,
                            UNDERGROUND_RIVER_MAXIMUM_DEPTH,
                            strength,
                        ),
                },
                position.y,
            );
        }

        selected
    }
}

fn lake_for_anchor(anchor: Vec3, seed: u64) -> Option<UndergroundLake> {
    if anchor.y - UNDERGROUND_LAKE_SURFACE_OFFSET < MINIMUM_WATER_Y
        || !anchor_has_lake(anchor, seed)
    {
        return None;
    }

    let hash = position_hash(anchor, seed ^ 0x1f83_d9ab_fb41_bd6b);
    let base_radius = lerp(
        UNDERGROUND_LAKE_MINIMUM_RADIUS,
        UNDERGROUND_LAKE_MAXIMUM_RADIUS,
        hash_unit(hash.rotate_left(11)),
    );
    let aspect = lerp(0.72, 1.28, hash_unit(hash.rotate_left(29)));

    Some(UndergroundLake {
        center: Vec2::new(anchor.x, anchor.z),
        radius: Vec2::new(base_radius * aspect, base_radius * (2.0 - aspect)),
        rotation: hash_unit(hash.rotate_left(43)) * std::f32::consts::TAU,
        shape_seed: hash.rotate_left(7),
        water_level: anchor.y - UNDERGROUND_LAKE_SURFACE_OFFSET,
        depth: lerp(
            UNDERGROUND_LAKE_MINIMUM_DEPTH,
            UNDERGROUND_LAKE_MAXIMUM_DEPTH,
            hash_unit(hash.rotate_left(53)),
        ),
    })
}

fn anchor_has_lake(anchor: Vec3, seed: u64) -> bool {
    if anchor.y - UNDERGROUND_LAKE_SURFACE_OFFSET < MINIMUM_WATER_Y {
        return false;
    }

    let hash = position_hash(anchor, seed ^ 0xa54f_f53a_5f1d_36f1);
    hash_unit(hash.rotate_left(19)) < UNDERGROUND_LAKE_CHANCE
}

fn consider_water(
    selected: &mut Option<UndergroundWaterSample>,
    candidate: UndergroundWaterSample,
    y: f32,
) {
    let cell_bottom = y - 0.5;
    let cell_top = y + 0.5;
    if cell_top <= candidate.bed_level || cell_bottom >= candidate.water_level {
        return;
    }

    let should_replace = selected.as_ref().is_none_or(|current| {
        candidate.water_level > current.water_level
            || ((candidate.water_level - current.water_level).abs() <= f32::EPSILON
                && candidate.bed_level < current.bed_level)
    });

    if should_replace {
        *selected = Some(candidate);
    }
}

fn irregular_boundary_scale(normalized: Vec2, seed: u64) -> f32 {
    let angle = normalized.y.atan2(normalized.x);
    let phase_a = hash_unit(seed) * std::f32::consts::TAU;
    let phase_b = hash_unit(seed.rotate_left(21)) * std::f32::consts::TAU;
    let phase_c = hash_unit(seed.rotate_left(43)) * std::f32::consts::TAU;
    let broad = (angle * 2.0 + phase_a).sin() * 0.16;
    let medium = (angle * 3.0 + phase_b).sin() * 0.10;
    let detail = (angle * 5.0 + phase_c).sin() * 0.06;

    (1.0 + broad + medium + detail).clamp(0.68, 1.32)
}

fn pair_hash(left: Vec3, right: Vec3, seed: u64) -> u64 {
    let (first, second) = if compare_position(&left, &right) != std::cmp::Ordering::Greater {
        (left, right)
    } else {
        (right, left)
    };
    let mut hash = position_hash(first, seed);

    for component in [second.x.to_bits(), second.y.to_bits(), second.z.to_bits()] {
        hash ^= component as u64;
        hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 31;
    }

    avalanche(hash)
}

fn position_hash(position: Vec3, seed: u64) -> u64 {
    let mut hash = seed;

    for component in [position.x.to_bits(), position.y.to_bits(), position.z.to_bits()] {
        hash ^= component as u64;
        hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 31;
    }

    avalanche(hash)
}

fn compare_position(left: &Vec3, right: &Vec3) -> std::cmp::Ordering {
    left.x
        .total_cmp(&right.x)
        .then_with(|| left.y.total_cmp(&right.y))
        .then_with(|| left.z.total_cmp(&right.z))
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
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
    fn underground_water_is_deterministic() {
        let anchors = [Vec3::new(12.5, 30.5, 8.5), Vec3::new(80.5, 18.5, 20.5)];
        let first = UndergroundWaterRegion::from_anchors(&anchors, 42);
        let second = UndergroundWaterRegion::from_anchors(&anchors, 42);

        assert_eq!(first.lakes.len(), second.lakes.len());
        for position in [
            Vec3::new(12.5, 28.5, 8.5),
            Vec3::new(80.5, 16.5, 20.5),
        ] {
            assert_eq!(
                first.water_at(position).is_some(),
                second.water_at(position).is_some(),
            );
        }
    }

    #[test]
    fn river_water_occupies_only_the_lower_part_of_a_connector() {
        let mut water = UndergroundWaterRegion::default();
        water.add_river_segment(
            Vec3::new(0.0, 20.0, 0.0),
            Vec3::new(20.0, 20.0, 0.0),
            6.0,
            6.0,
        );

        assert!(water.water_at(Vec3::new(10.0, 17.5, 0.0)).is_some());
        assert!(water.water_at(Vec3::new(10.0, 21.0, 0.0)).is_none());
    }

    #[test]
    fn steep_underground_river_segment_forms_a_waterfall() {
        let mut water = UndergroundWaterRegion::default();
        water.add_river_segment(
            Vec3::new(0.0, 30.0, 0.0),
            Vec3::new(5.0, 18.0, 0.0),
            6.0,
            6.0,
        );

        assert!(!water.waterfalls.is_empty());
        let midpoint = water.waterfalls[0].from.lerp(water.waterfalls[0].to, 0.5);
        assert!(water.water_at(midpoint).is_some());
    }

    #[test]
    fn lake_anchor_always_drains_into_a_lower_connection() {
        let mut chosen = None;
        for x in 0..128 {
            let anchor = Vec3::new(x as f32 + 0.5, 40.5, 0.5);
            if anchor_has_lake(anchor, 42) {
                chosen = Some(anchor);
                break;
            }
        }
        let high = chosen.expect("test should find a deterministic lake anchor");
        let low = high + Vec3::new(20.0, -12.0, 0.0);

        assert!(UndergroundWaterRegion::connection_carries_water(high, low, 42));
    }
}
