use bevy::prelude::*;

use crate::content::{
    biome_hydrology::BiomeHydrology,
    dimension_hydrology::DimensionHydrology,
};

use super::feature_graph::FeatureGraph;

const HYDROLOGY_REGION_SIZE: f32 = 128.0;
const MACRO_SAMPLE_GRID: usize = 5;

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

const SHORE_STRENGTH: f32 = 0.25;
const BED_MATERIAL_DEPTH: f32 = 1.5;

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

#[derive(Clone, Copy, Debug)]
pub struct HydrologySurfaceSample {
    pub elevation: f32,
    pub continentalness: f32,
    pub biome_hydrology: BiomeHydrology,
}

#[derive(Clone, Copy, Debug)]
pub struct HydrologyBiomeOverlay<'a> {
    pub surface_weight: f32,
    pub coast_biome: Option<&'a str>,
    pub coast_weight: f32,
    pub ocean_biome: Option<&'a str>,
    pub ocean_weight: f32,
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
    biome_hydrology: BiomeHydrology,
}

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub coord: IVec2,
    pub terrain: HydrologyTerrainSummary,
    pub river_graph: FeatureGraph,
    pub river_carve_depth: f32,
    pub water_bodies: Vec<WaterBody>,
    sea_level: f32,
    settings: DimensionHydrology,
    macro_samples: Vec<HydrologyMacroSample>,
}

impl HydrologyRegion {
    fn empty(coord: IVec2, sea_level: f32, settings: DimensionHydrology) -> Self {
        Self {
            coord,
            terrain: HydrologyTerrainSummary::default(),
            river_graph: FeatureGraph::default(),
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies: Vec::new(),
            sea_level,
            settings,
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

                if strength <= 0.0
                    || position.y < bottom - 0.5
                    || position.y > body.water_level + 0.5
                {
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
                    fluid_id: self.settings.water_fluid.as_str(),
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
                fluid_id: self.settings.water_fluid.as_str(),
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

    pub fn solid_block_at(&self, position: Vec3) -> Option<&str> {
        let horizontal = Vec2::new(position.x, position.z);

        if let Some((body, strength)) = self
            .water_bodies
            .iter()
            .filter_map(|body| {
                let strength = body.horizontal_strength(horizontal);
                (strength > 0.0).then_some((body, strength))
            })
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
        {
            let bottom = body.water_level - body.carve_depth * strength;

            if position.y >= bottom - BED_MATERIAL_DEPTH
                && position.y <= bottom + BED_MATERIAL_DEPTH
            {
                return self.material_for_water_body(body.kind, strength);
            }
        }

        if let Some(river) = self.river_graph.sample_horizontal(horizontal) {
            let profile = smoothstep(river.strength);
            let bed = river.height - self.river_carve_depth * profile;

            if position.y >= bed - BED_MATERIAL_DEPTH
                && position.y <= bed + BED_MATERIAL_DEPTH
            {
                if river.strength <= SHORE_STRENGTH {
                    return self
                        .settings
                        .shore_block
                        .as_deref()
                        .or(self.settings.river_bed_block.as_deref());
                }

                return self
                    .settings
                    .river_bed_block
                    .as_deref()
                    .or(self.settings.shore_block.as_deref());
            }
        }

        let strength = self.ocean_strength_at(horizontal);

        if strength > 0.0 {
            let sample = self.macro_sample_at(horizontal)?;
            let target_floor =
                self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
            let floor = lerp(sample.elevation, target_floor, strength);

            if position.y >= floor - BED_MATERIAL_DEPTH
                && position.y <= floor + BED_MATERIAL_DEPTH
            {
                if strength <= SHORE_STRENGTH {
                    return self
                        .settings
                        .shore_block
                        .as_deref()
                        .or(self.settings.ocean_bed_block.as_deref());
                }

                return self
                    .settings
                    .ocean_bed_block
                    .as_deref()
                    .or(self.settings.shore_block.as_deref());
            }
        }

        None
    }

    pub fn ocean_strength_at(&self, position: Vec2) -> f32 {
        self.macro_sample_at(position)
            .map_or(0.0, |sample| ocean_strength(sample.continentalness))
    }

    fn material_for_water_body(&self, kind: WaterBodyKind, strength: f32) -> Option<&str> {
        let bed = match kind {
            WaterBodyKind::Lake => self.settings.lake_bed_block.as_deref(),
            WaterBodyKind::Ocean => self.settings.ocean_bed_block.as_deref(),
        };

        if strength <= SHORE_STRENGTH {
            self.settings.shore_block.as_deref().or(bed)
        } else {
            bed.or(self.settings.shore_block.as_deref())
        }
    }

    fn ocean_density_delta(&self, position: Vec2) -> f32 {
        let Some(sample) = self.macro_sample_at(position) else {
            return 0.0;
        };
        let strength = self.ocean_strength_at(position);

        if strength <= 0.0 {
            return 0.0;
        }

        let target_floor =
            self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
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

#[derive(Clone, Debug)]
pub struct HydrologyField {
    seed: u64,
    sea_level: i32,
    settings: DimensionHydrology,
}

impl HydrologyField {
    pub fn new(seed: u64, sea_level: i32, settings: DimensionHydrology) -> Self {
        Self {
            seed,
            sea_level,
            settings,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub fn biome_overlay(&self, continentalness: f32) -> HydrologyBiomeOverlay<'_> {
        let strength = ocean_strength(continentalness);
        let mut surface_weight = (1.0 - strength * 2.0).clamp(0.0, 1.0);
        let mut coast_weight = (1.0 - (strength * 2.0 - 1.0).abs()).clamp(0.0, 1.0);
        let mut ocean_weight = (strength * 2.0 - 1.0).clamp(0.0, 1.0);
        let coast_biome = self.settings.coast_biome.as_deref();
        let ocean_biome = self.settings.ocean_biome.as_deref();

        if coast_biome.is_none() {
            if strength < 0.5 {
                surface_weight += coast_weight;
            } else {
                ocean_weight += coast_weight;
            }
            coast_weight = 0.0;
        }

        if ocean_biome.is_none() {
            if coast_biome.is_some() {
                coast_weight += ocean_weight;
            } else {
                surface_weight += ocean_weight;
            }
            ocean_weight = 0.0;
        }

        let total = surface_weight + coast_weight + ocean_weight;

        if total > f32::EPSILON {
            surface_weight /= total;
            coast_weight /= total;
            ocean_weight /= total;
        }

        HydrologyBiomeOverlay {
            surface_weight,
            coast_biome,
            coast_weight,
            ocean_biome,
            ocean_weight,
        }
    }

    pub fn region(&self, coord: IVec2) -> HydrologyRegion {
        HydrologyRegion::empty(
            coord,
            self.sea_level as f32,
            self.settings.clone(),
        )
    }

    pub fn region_from_macro_terrain(
        &self,
        coord: IVec2,
        mut sample: impl FnMut(Vec2) -> HydrologySurfaceSample,
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
                let surface = sample(position);

                minimum_elevation = minimum_elevation.min(surface.elevation);
                maximum_elevation = maximum_elevation.max(surface.elevation);
                elevation_sum += surface.elevation;
                continentalness_sum += surface.continentalness;
                count += 1.0;
                macro_samples.push(HydrologyMacroSample {
                    elevation: surface.elevation,
                    continentalness: surface.continentalness,
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
                        && source.biome_hydrology.can_generate_river
                        && downstream.biome_hydrology.can_generate_river
                        && edge_intersects_region(coord, source.position, downstream.position)
                    {
                        let hash = cell_hash(source_cell, self.seed ^ 0x6a09_e667_f3bc_c909);
                        let base_radius = lerp(
                            RIVER_MINIMUM_RADIUS,
                            RIVER_MAXIMUM_RADIUS,
                            hash_unit(hash),
                        );
                        let width_multiplier = (source.biome_hydrology.river_width_multiplier
                            + downstream.biome_hydrology.river_width_multiplier)
                            * 0.5;
                        let radius = base_radius * width_multiplier;

                        if radius > f32::EPSILON {
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
                    }
                } else if let Some(lake) = lake_for_local_basin(
                    source_cell,
                    source,
                    &neighbors,
                    self.seed,
                    self.sea_level as f32,
                    &self.settings.water_fluid,
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
            settings: self.settings.clone(),
            macro_samples,
        }
    }
}

fn drainage_node(
    cell: IVec2,
    seed: u64,
    sample: &mut impl FnMut(Vec2) -> HydrologySurfaceSample,
) -> DrainageNode {
    let position = drainage_position(cell, seed);
    let surface = sample(position);

    DrainageNode {
        position,
        elevation: surface.elevation,
        continentalness: surface.continentalness,
        biome_hydrology: surface.biome_hydrology,
    }
}

fn drainage_neighbors(
    cell: IVec2,
    seed: u64,
    sample: &mut impl FnMut(Vec2) -> HydrologySurfaceSample,
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
    water_fluid: &str,
) -> Option<WaterBody> {
    if !source.biome_hydrology.can_generate_lake
        || source.elevation <= sea_level + 1.0
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
    let lake_chance =
        (LAKE_CHANCE * source.biome_hydrology.lake_chance_multiplier).clamp(0.0, 1.0);

    if hash_unit(hash.rotate_left(17)) > lake_chance {
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
        fluid_id: water_fluid.to_owned(),
    })
}

fn ocean_strength(continentalness: f32) -> f32 {
    let raw = (OCEAN_CONTINENTALNESS_THRESHOLD - continentalness) / OCEAN_TRANSITION_WIDTH;
    smoothstep(raw.clamp(0.0, 1.0))
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

    fn settings() -> DimensionHydrology {
        DimensionHydrology {
            ocean_biome: Some("mineclone:test/ocean".into()),
            coast_biome: Some("mineclone:test/coast".into()),
            ..default()
        }
    }

    fn field() -> HydrologyField {
        HydrologyField::new(42, 64, settings())
    }

    fn surface(elevation: f32, continentalness: f32) -> HydrologySurfaceSample {
        HydrologySurfaceSample {
            elevation,
            continentalness,
            biome_hydrology: BiomeHydrology::default(),
        }
    }

    #[test]
    fn water_body_strength_fades_to_zero_at_shoreline() {
        let body = WaterBody {
            kind: WaterBodyKind::Lake,
            center: Vec2::ZERO,
            radius: Vec2::splat(10.0),
            water_level: 64.0,
            carve_depth: 8.0,
            fluid_id: settings().water_fluid,
        };

        assert_eq!(body.horizontal_strength(Vec2::ZERO), 1.0);
        assert_eq!(body.horizontal_strength(Vec2::new(10.0, 0.0)), 0.0);
    }

    #[test]
    fn macro_terrain_summary_is_deterministic() {
        let field = field();
        let sample = |position: Vec2| surface(position.x + position.y, 0.5);
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
        let field = field();
        let region = field.region_from_macro_terrain(IVec2::ZERO, |_| surface(70.0, 0.1));
        let center = Vec2::splat(HYDROLOGY_REGION_SIZE * 0.5);

        let water = region.water_at(center).unwrap();
        assert_eq!(water.fluid_id, field.settings.water_fluid);
        assert_eq!(water.water_level, 64.0);
        assert!(region.density_delta(Vec3::new(center.x, 60.0, center.y)) < 0.0);
    }

    #[test]
    fn hydrology_biome_overlay_transitions_surface_to_coast_to_ocean() {
        let field = field();
        let land = field.biome_overlay(0.5);
        let coast = field.biome_overlay(0.28);
        let ocean = field.biome_overlay(0.1);

        assert_eq!(land.surface_weight, 1.0);
        assert!(coast.coast_weight > coast.surface_weight);
        assert!(coast.coast_weight > coast.ocean_weight);
        assert_eq!(ocean.ocean_weight, 1.0);
    }

    #[test]
    fn biome_can_disable_lake_generation() {
        let source = DrainageNode {
            position: Vec2::ZERO,
            elevation: 80.0,
            continentalness: 0.8,
            biome_hydrology: BiomeHydrology {
                can_generate_lake: false,
                lake_chance_multiplier: 10.0,
                ..default()
            },
        };
        let neighbors = vec![DrainageNode {
            position: Vec2::X,
            elevation: 84.0,
            continentalness: 0.8,
            biome_hydrology: BiomeHydrology::default(),
        }];

        assert!(
            lake_for_local_basin(
                IVec2::ZERO,
                source,
                &neighbors,
                42,
                64.0,
                "mineclone:water",
            )
            .is_none()
        );
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
