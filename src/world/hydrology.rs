use bevy::prelude::*;

use super::feature_graph::FeatureGraph;

const HYDROLOGY_REGION_SIZE: f32 = 128.0;
const MACRO_SAMPLE_GRID: usize = 5;
const DEFAULT_WATER_FLUID: &str = "mineclone:water";

const OCEAN_CONTINENTALNESS_THRESHOLD: f32 = 0.34;
const OCEAN_TRANSITION_WIDTH: f32 = 0.12;
const OCEAN_MINIMUM_DEPTH: f32 = 8.0;
const OCEAN_EXTRA_DEPTH: f32 = 18.0;

const RIVER_SOURCE_MARGIN_CELLS: i32 = 1;
const RIVER_MINIMUM_DROP: f32 = 0.75;
const RIVER_MINIMUM_RADIUS: f32 = 3.0;
const RIVER_MAXIMUM_RADIUS: f32 = 6.0;
const RIVER_CARVE_DEPTH: f32 = 4.0;
const RIVER_CARVE_STRENGTH: f32 = 8.0;

const LAKE_MINIMUM_RADIUS: f32 = 16.0;
const LAKE_MAXIMUM_RADIUS: f32 = 34.0;
const LAKE_CARVE_DEPTH: f32 = 6.0;
const LAKE_MINIMUM_RELIEF: f32 = 1.0;
const LAKE_CHANCE: f32 = 0.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterBodyKind {
    Lake,
    Ocean,
}

#[derive(Clone, Debug)]
pub struct WaterBody {
    pub kind: WaterBodyKind,
    pub center: Vec2,
    pub radius: Vec2,
    pub water_level: f32,
    pub carve_depth: f32,
    pub fluid_id: String,
}

impl WaterBody {
    pub fn horizontal_strength(&self, position: Vec2) -> f32 {
        let delta = position - self.center;
        let normalized = Vec2::new(delta.x / self.radius.x, delta.y / self.radius.y);
        let distance = normalized.length();

        smoothstep(1.0 - distance.clamp(0.0, 1.0))
    }

    pub fn contains_horizontal(&self, position: Vec2) -> bool {
        self.horizontal_strength(position) > 0.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologyWaterSample<'a> {
    pub fluid_id: &'a str,
    pub water_level: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HydrologyTerrainSummary {
    pub minimum_elevation: f32,
    pub maximum_elevation: f32,
    pub mean_elevation: f32,
    pub mean_continentalness: f32,
}

#[derive(Clone, Copy, Debug)]
struct HydrologyMacroSample {
    elevation: f32,
    continentalness: f32,
}

#[derive(Clone, Copy, Debug)]
struct DrainageNode {
    position: Vec2,
    elevation: f32,
    continentalness: f32,
}

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub coord: IVec2,
    pub terrain: HydrologyTerrainSummary,
    pub river_graph: FeatureGraph,
    pub river_carve_depth: f32,
    pub water_bodies: Vec<WaterBody>,
    sea_level: f32,
    macro_samples: Vec<HydrologyMacroSample>,
}

impl HydrologyRegion {
    fn empty(coord: IVec2, sea_level: f32) -> Self {
        Self {
            coord,
            terrain: HydrologyTerrainSummary::default(),
            river_graph: FeatureGraph::default(),
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies: Vec::new(),
            sea_level,
            macro_samples: Vec::new(),
        }
    }

    pub fn density_delta(&self, position: Vec3) -> f32 {
        let horizontal = Vec2::new(position.x, position.z);
        let ocean_delta = self.ocean_density_delta(horizontal);
        let river_delta = self
            .river_graph
            .sample_horizontal(horizontal)
            .map_or(0.0, |sample| {
                let profile = smoothstep(sample.strength);
                let bed = sample.height - self.river_carve_depth * profile;

                if position.y < bed - 0.5 || position.y > sample.height + 1.5 {
                    return 0.0;
                }

                -RIVER_CARVE_STRENGTH * profile
            });
        let water_body_delta = self
            .water_bodies
            .iter()
            .map(|body| {
                if body.carve_depth <= 0.0 {
                    return 0.0;
                }

                let strength = body.horizontal_strength(horizontal);
                let bottom = body.water_level - body.carve_depth * strength;

                if strength <= 0.0 || position.y < bottom - 0.5 || position.y > body.water_level + 0.5 {
                    return 0.0;
                }

                -(body.carve_depth + 2.0) * strength
            })
            .sum::<f32>();

        ocean_delta + river_delta + water_body_delta
    }

    pub fn water_at(&self, position: Vec2) -> Option<HydrologyWaterSample<'_>> {
        let mut selected = self
            .water_bodies
            .iter()
            .filter(|body| body.contains_horizontal(position))
            .max_by(|left, right| left.water_level.total_cmp(&right.water_level))
            .map(|body| HydrologyWaterSample {
                fluid_id: body.fluid_id.as_str(),
                water_level: body.water_level,
            });

        if let Some(river) = self.river_graph.sample_horizontal(position) {
            if river.strength > 0.0 {
                let candidate = HydrologyWaterSample {
                    fluid_id: DEFAULT_WATER_FLUID,
                    water_level: river.height,
                };

                if selected
                    .as_ref()
                    .is_none_or(|current| candidate.water_level > current.water_level)
                {
                    selected = Some(candidate);
                }
            }
        }

        if self.ocean_strength_at(position) > 0.0 {
            let candidate = HydrologyWaterSample {
                fluid_id: DEFAULT_WATER_FLUID,
                water_level: self.sea_level,
            };

            if selected
                .as_ref()
                .is_none_or(|current| candidate.water_level > current.water_level)
            {
                selected = Some(candidate);
            }
        }

        selected
    }

    pub fn ocean_strength_at(&self, position: Vec2) -> f32 {
        let Some(sample) = self.macro_sample_at(position) else {
            return 0.0;
        };
        let raw = (OCEAN_CONTINENTALNESS_THRESHOLD - sample.continentalness)
            / OCEAN_TRANSITION_WIDTH;

        smoothstep(raw.clamp(0.0, 1.0))
    }

    fn ocean_density_delta(&self, position: Vec2) -> f32 {
        let Some(sample) = self.macro_sample_at(position) else {
            return 0.0;
        };
        let strength = self.ocean_strength_at(position);

        if strength <= 0.0 {
            return 0.0;
        }

        let target_floor = self.sea_level
            - OCEAN_MINIMUM_DEPTH
            - OCEAN_EXTRA_DEPTH * strength;
        (target_floor - sample.elevation).min(0.0) * strength
    }

    fn macro_sample_at(&self, position: Vec2) -> Option<HydrologyMacroSample> {
        if self.macro_samples.len() != MACRO_SAMPLE_GRID * MACRO_SAMPLE_GRID {
            return None;
        }

        let origin = self.coord.as_vec2() * HYDROLOGY_REGION_SIZE;
        let step = HYDROLOGY_REGION_SIZE / (MACRO_SAMPLE_GRID - 1) as f32;
        let local = (position - origin) / step;
        let x = local.x.clamp(0.0, (MACRO_SAMPLE_GRID - 1) as f32);
        let z = local.y.clamp(0.0, (MACRO_SAMPLE_GRID - 1) as f32);
        let x0 = x.floor() as usize;
        let z0 = z.floor() as usize;
        let x1 = (x0 + 1).min(MACRO_SAMPLE_GRID - 1);
        let z1 = (z0 + 1).min(MACRO_SAMPLE_GRID - 1);
        let tx = smoothstep(x - x0 as f32);
        let tz = smoothstep(z - z0 as f32);
        let top = interpolate_macro(
            self.macro_samples[macro_index(x0, z0)],
            self.macro_samples[macro_index(x1, z0)],
            tx,
        );
        let bottom = interpolate_macro(
            self.macro_samples[macro_index(x0, z1)],
            self.macro_samples[macro_index(x1, z1)],
            tx,
        );

        Some(interpolate_macro(top, bottom, tz))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologyField {
    seed: u64,
    sea_level: i32,
}

impl HydrologyField {
    pub fn new(seed: u64, sea_level: i32) -> Self {
        Self { seed, sea_level }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub fn region(&self, coord: IVec2) -> HydrologyRegion {
        HydrologyRegion::empty(coord, self.sea_level as f32)
    }

    pub fn region_from_macro_terrain(
        &self,
        coord: IVec2,
        mut sample: impl FnMut(Vec2) -> (f32, f32),
    ) -> HydrologyRegion {
        let mut minimum_elevation = f32::MAX;
        let mut maximum_elevation = f32::MIN;
        let mut elevation_sum = 0.0;
        let mut continentalness_sum = 0.0;
        let mut count = 0.0;
        let mut macro_samples = Vec::with_capacity(MACRO_SAMPLE_GRID * MACRO_SAMPLE_GRID);

        for z in 0..MACRO_SAMPLE_GRID {
            for x in 0..MACRO_SAMPLE_GRID {
                let position = macro_sample_position(coord, x, z);
                let (elevation, continentalness) = sample(position);

                minimum_elevation = minimum_elevation.min(elevation);
                maximum_elevation = maximum_elevation.max(elevation);
                elevation_sum += elevation;
                continentalness_sum += continentalness;
                count += 1.0;
                macro_samples.push(HydrologyMacroSample {
                    elevation,
                    continentalness,
                });
            }
        }

        let mut river_graph = FeatureGraph::default();
        let mut water_bodies = Vec::new();

        for dz in -RIVER_SOURCE_MARGIN_CELLS..=RIVER_SOURCE_MARGIN_CELLS {
            for dx in -RIVER_SOURCE_MARGIN_CELLS..=RIVER_SOURCE_MARGIN_CELLS {
                let source_cell = coord + IVec2::new(dx, dz);
                let source = drainage_node(source_cell, self.seed, &mut sample);
                let neighbors = drainage_neighbors(source_cell, self.seed, &mut sample);

                if let Some(downstream) = select_downstream(source, &neighbors) {
                    if source.continentalness > OCEAN_CONTINENTALNESS_THRESHOLD
                        && edge_intersects_region(coord, source.position, downstream.position)
                    {
                        let hash = cell_hash(source_cell, self.seed ^ 0x6a09_e667_f3bc_c909);
                        let radius = lerp(
                            RIVER_MINIMUM_RADIUS,
                            RIVER_MAXIMUM_RADIUS,
                            hash_unit(hash),
                        );
                        let from = river_graph.add_node(Vec3::new(
                            source.position.x,
                            (source.elevation - 0.75).max(1.0),
                            source.position.y,
                        ));
                        let to = river_graph.add_node(Vec3::new(
                            downstream.position.x,
                            (downstream.elevation - 0.75).max(1.0),
                            downstream.position.y,
                        ));
                        river_graph.add_edge(from, to, radius, radius * 1.15);
                    }
                } else if let Some(lake) = lake_for_local_basin(
                    source_cell,
                    source,
                    &neighbors,
                    self.seed,
                    self.sea_level as f32,
                ) {
                    if water_body_intersects_region(coord, &lake) {
                        water_bodies.push(lake);
                    }
                }
            }
        }

        HydrologyRegion {
            coord,
            terrain: HydrologyTerrainSummary {
                minimum_elevation,
                maximum_elevation,
                mean_elevation: elevation_sum / count,
                mean_continentalness: continentalness_sum / count,
            },
            river_graph,
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies,
            sea_level: self.sea_level as f32,
            macro_samples,
        }
    }
}

fn drainage_node(
    cell: IVec2,
    seed: u64,
    sample: &mut impl FnMut(Vec2) -> (f32, f32),
) -> DrainageNode {
    let position = drainage_position(cell, seed);
    let (elevation, continentalness) = sample(position);

    DrainageNode {
        position,
        elevation,
        continentalness,
    }
}

fn drainage_neighbors(
    cell: IVec2,
    seed: u64,
    sample: &mut impl FnMut(Vec2) -> (f32, f32),
) -> Vec<DrainageNode> {
    let mut neighbors = Vec::with_capacity(8);

    for dz in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dz == 0 {
                continue;
            }

            neighbors.push(drainage_node(cell + IVec2::new(dx, dz), seed, sample));
        }
    }

    neighbors
}

fn select_downstream(source: DrainageNode, neighbors: &[DrainageNode]) -> Option<DrainageNode> {
    let downstream = neighbors
        .iter()
        .copied()
        .min_by(|left, right| left.elevation.total_cmp(&right.elevation))?;

    (downstream.elevation + RIVER_MINIMUM_DROP < source.elevation).then_some(downstream)
}

fn lake_for_local_basin(
    cell: IVec2,
    source: DrainageNode,
    neighbors: &[DrainageNode],
    seed: u64,
    sea_level: f32,
) -> Option<WaterBody> {
    if source.elevation <= sea_level + 1.0
        || source.continentalness <= OCEAN_CONTINENTALNESS_THRESHOLD
    {
        return None;
    }

    let spill = neighbors
        .iter()
        .map(|neighbor| neighbor.elevation)
        .min_by(f32::total_cmp)?;
    let relief = spill - source.elevation;

    if relief < LAKE_MINIMUM_RELIEF {
        return None;
    }

    let hash = cell_hash(cell, seed ^ 0xbb67_ae85_84ca_a73b);

    if hash_unit(hash.rotate_left(17)) > LAKE_CHANCE {
        return None;
    }

    let radius_x = lerp(
        LAKE_MINIMUM_RADIUS,
        LAKE_MAXIMUM_RADIUS,
        hash_unit(hash.rotate_left(29)),
    );
    let radius_z = lerp(
        LAKE_MINIMUM_RADIUS,
        LAKE_MAXIMUM_RADIUS,
        hash_unit(hash.rotate_left(43)),
    );
    let water_level = source.elevation + relief.min(5.0) * 0.7;

    Some(WaterBody {
        kind: WaterBodyKind::Lake,
        center: source.position,
        radius: Vec2::new(radius_x, radius_z),
        water_level,
        carve_depth: LAKE_CARVE_DEPTH,
        fluid_id: DEFAULT_WATER_FLUID.into(),
    })
}

fn drainage_position(cell: IVec2, seed: u64) -> Vec2 {
    let base = (cell.as_vec2() + Vec2::splat(0.5)) * HYDROLOGY_REGION_SIZE;
    let hash = cell_hash(cell, seed);
    let jitter = Vec2::new(
        hash_signed(hash) * HYDROLOGY_REGION_SIZE * 0.22,
        hash_signed(hash.rotate_left(31)) * HYDROLOGY_REGION_SIZE * 0.22,
    );

    base + jitter
}

fn edge_intersects_region(coord: IVec2, from: Vec2, to: Vec2) -> bool {
    let (minimum, maximum) = region_bounds(coord);
    let margin = RIVER_MAXIMUM_RADIUS;
    let edge_minimum = from.min(to) - Vec2::splat(margin);
    let edge_maximum = from.max(to) + Vec2::splat(margin);

    edge_maximum.x >= minimum.x
        && edge_minimum.x <= maximum.x
        && edge_maximum.y >= minimum.y
        && edge_minimum.y <= maximum.y
}

fn water_body_intersects_region(coord: IVec2, body: &WaterBody) -> bool {
    let (minimum, maximum) = region_bounds(coord);
    let body_minimum = body.center - body.radius;
    let body_maximum = body.center + body.radius;

    body_maximum.x >= minimum.x
        && body_minimum.x <= maximum.x
        && body_maximum.y >= minimum.y
        && body_minimum.y <= maximum.y
}

fn region_bounds(coord: IVec2) -> (Vec2, Vec2) {
    let minimum = coord.as_vec2() * HYDROLOGY_REGION_SIZE;
    (minimum, minimum + Vec2::splat(HYDROLOGY_REGION_SIZE))
}

fn macro_sample_position(coord: IVec2, x: usize, z: usize) -> Vec2 {
    let origin = coord.as_vec2() * HYDROLOGY_REGION_SIZE;
    let step = HYDROLOGY_REGION_SIZE / (MACRO_SAMPLE_GRID - 1) as f32;

    origin + Vec2::new(x as f32 * step, z as f32 * step)
}

fn macro_index(x: usize, z: usize) -> usize {
    x + z * MACRO_SAMPLE_GRID
}

fn interpolate_macro(
    from: HydrologyMacroSample,
    to: HydrologyMacroSample,
    amount: f32,
) -> HydrologyMacroSample {
    HydrologyMacroSample {
        elevation: lerp(from.elevation, to.elevation, amount),
        continentalness: lerp(from.continentalness, to.continentalness, amount),
    }
}

fn cell_hash(cell: IVec2, seed: u64) -> u64 {
    let mut hash = seed ^ 0x9e37_79b9_7f4a_7c15;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^ (hash >> 31)
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

fn hash_signed(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
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
    fn water_body_strength_fades_to_zero_at_shoreline() {
        let body = WaterBody {
            kind: WaterBodyKind::Lake,
            center: Vec2::ZERO,
            radius: Vec2::splat(10.0),
            water_level: 64.0,
            carve_depth: 8.0,
            fluid_id: DEFAULT_WATER_FLUID.into(),
        };

        assert_eq!(body.horizontal_strength(Vec2::ZERO), 1.0);
        assert_eq!(body.horizontal_strength(Vec2::new(10.0, 0.0)), 0.0);
    }

    #[test]
    fn macro_terrain_summary_is_deterministic() {
        let field = HydrologyField::new(42, 64);
        let sample = |position: Vec2| (position.x + position.y, 0.5);
        let first = field.region_from_macro_terrain(IVec2::ZERO, sample);
        let second = field.region_from_macro_terrain(IVec2::ZERO, sample);

        assert_eq!(first.terrain.minimum_elevation, second.terrain.minimum_elevation);
        assert_eq!(first.terrain.maximum_elevation, second.terrain.maximum_elevation);
        assert_eq!(first.terrain.mean_elevation, second.terrain.mean_elevation);
        assert_eq!(first.terrain.mean_continentalness, 0.5);
        assert_eq!(first.river_graph.edges().len(), second.river_graph.edges().len());
        assert_eq!(first.water_bodies.len(), second.water_bodies.len());
    }

    #[test]
    fn low_continentalness_produces_ocean_water_and_carving() {
        let field = HydrologyField::new(42, 64);
        let region = field.region_from_macro_terrain(IVec2::ZERO, |_| (70.0, 0.1));
        let center = Vec2::splat(HYDROLOGY_REGION_SIZE * 0.5);

        let water = region.water_at(center).unwrap();
        assert_eq!(water.fluid_id, DEFAULT_WATER_FLUID);
        assert_eq!(water.water_level, 64.0);
        assert!(region.density_delta(Vec3::new(center.x, 60.0, center.y)) < 0.0);
    }

    #[test]
    fn adjacent_regions_sample_the_same_shared_boundary() {
        let left_boundary = macro_sample_position(IVec2::ZERO, MACRO_SAMPLE_GRID - 1, 2);
        let right_boundary = macro_sample_position(IVec2::X, 0, 2);

        assert_eq!(left_boundary, right_boundary);
    }

    #[test]
    fn drainage_position_is_global_and_region_independent() {
        let cell = IVec2::new(3, -4);

        assert_eq!(drainage_position(cell, 99), drainage_position(cell, 99));
    }
}
