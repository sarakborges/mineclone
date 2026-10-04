use std::sync::{Arc, RwLock};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    content::dimension::DimensionBiomeSizeAxis,
    world::deterministic::hash_signed,
};

use super::{
    BiomeField, BiomeFieldEntry, SurfaceBoundarySample,
    spatial::{cell_hash, hash_unit, warp_surface_position},
};

const SURFACE_MAP_CELL_SIZE: f32 = 8.0;
const SURFACE_SITE_SEARCH_RADIUS: i32 = 4;
const SURFACE_ROW_JITTER_FRACTION: f32 = 0.32;
const BASE_HASH_SALT: u64 = 0x6a09_e667_f3bc_c909;
const ANCHOR_HASH_SALT: u64 = 0xbb67_ae85_84ca_a73b;
const SHAPE_HASH_SALT: u64 = 0x3c6e_f372_fe94_f82b;
const STAMP_ANCHOR_STRIDE_CELLS: i32 = 16;
const STAMP_ANCHOR_JITTER_CELLS: i32 = 2;
const STAMP_POSITION_OFFSET_CELLS: i32 = 2;
const MAX_BIOME_CANDIDATES_PER_ANCHOR: usize = 16;
const SHAPE_ANGLE_SECTORS: i32 = 24;
const SHAPE_IRREGULARITY_LEVELS: [f32; 5] = [0.34, 0.24, 0.14, 0.07, 0.0];

#[derive(Clone)]
pub(super) struct SurfaceFieldConfig {
    pub(super) land_spacing: Vec2,
    state: Arc<RwLock<SurfaceBiomeMapState>>,
}

impl SurfaceFieldConfig {
    pub(super) fn from_biomes(
        _surface_biomes: &[BiomeFieldEntry],
        _ocean_surface_index: Option<usize>,
    ) -> Self {
        Self {
            land_spacing: Vec2::splat(SURFACE_MAP_CELL_SIZE),
            state: Arc::new(RwLock::new(SurfaceBiomeMapState::default())),
        }
    }

    fn ensure_planned_through(&self, field: &BiomeField, required_cell_radius: i32) {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.spawn_oceans != Some(field.spawn_oceans) {
            *state = SurfaceBiomeMapState {
                spawn_oceans: Some(field.spawn_oceans),
                ..default()
            };
        }
        ensure_initialized(field, &mut state);

        let reach = maximum_stamp_reach_cells(field);
        let required_anchor_radius = div_ceil_i32(
            required_cell_radius
                .saturating_add(reach)
                .saturating_add(STAMP_ANCHOR_JITTER_CELLS)
                .saturating_add(STAMP_POSITION_OFFSET_CELLS),
            STAMP_ANCHOR_STRIDE_CELLS,
        )
        .saturating_add(1);

        while state.planned_anchor_radius < required_anchor_radius {
            let next_radius = state.planned_anchor_radius + 1;
            plan_anchor_ring(field, &mut state, next_radius);
            state.planned_anchor_radius = next_radius;
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SurfaceBiomeCell {
    biome_index: usize,
}

struct StampShape {
    cells: Vec<IVec2>,
    membership: HashSet<IVec2>,
    minimum: IVec2,
    maximum: IVec2,
}

struct SurfaceBiomeMapState {
    spawn_oceans: Option<bool>,
    planned_anchor_radius: i32,
    base_biome_index: Option<usize>,
    cells: HashMap<IVec2, SurfaceBiomeCell>,
    placed_counts: Vec<u32>,
    attempted_anchors: u64,
    painted_minimum: Option<IVec2>,
    painted_maximum: Option<IVec2>,
}

impl Default for SurfaceBiomeMapState {
    fn default() -> Self {
        Self {
            spawn_oceans: None,
            planned_anchor_radius: -1,
            base_biome_index: None,
            cells: HashMap::new(),
            placed_counts: Vec::new(),
            attempted_anchors: 0,
            painted_minimum: None,
            painted_maximum: None,
        }
    }
}

impl SurfaceBiomeMapState {
    fn biome_at(&self, cell: IVec2) -> usize {
        self.cells
            .get(&cell)
            .map(|resolved| resolved.biome_index)
            .or(self.base_biome_index)
            .unwrap_or(0)
    }

    fn is_simulated_base_cell(&self, cell: IVec2, candidate: &HashSet<IVec2>) -> bool {
        if candidate.contains(&cell) {
            return false;
        }
        let Some(base_index) = self.base_biome_index else {
            return true;
        };
        self.cells
            .get(&cell)
            .is_none_or(|resolved| resolved.biome_index == base_index)
    }

    fn non_base_region_count(&self) -> u32 {
        let Some(base_index) = self.base_biome_index else {
            return 0;
        };
        self.placed_counts
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != base_index)
            .map(|(_, count)| *count)
            .sum()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SurfaceFieldSample {
    pub(super) primary_index: usize,
    pub(super) primary_terrain_strength: f32,
    pub(super) boundary: Option<SurfaceBoundarySample>,
}

impl BiomeField {
    pub(super) fn surface_biome_index_at(&self, position: Vec2) -> usize {
        if let Some(index) = self.single_surface_biome {
            return index;
        }

        self.surface_field_sample_at(position).primary_index
    }

    pub(super) fn surface_field_sample_at(&self, position: Vec2) -> SurfaceFieldSample {
        if let Some(index) = self.single_surface_biome {
            return SurfaceFieldSample {
                primary_index: index,
                primary_terrain_strength: 1.0,
                boundary: None,
            };
        }

        let warped = warp_surface_position(position, self.seed);
        let spacing = self.surface_field_config.land_spacing;
        let center = IVec2::new(
            (warped.x / spacing.x).round() as i32,
            (warped.y / spacing.y).round() as i32,
        );
        let required_radius = center
            .x
            .abs()
            .max(center.y.abs())
            .saturating_add(SURFACE_SITE_SEARCH_RADIUS);
        self.surface_field_config
            .ensure_planned_through(self, required_radius);

        let state = self
            .surface_field_config
            .state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let mut primary_cell = center;
        let mut primary_site = Vec2::ZERO;
        let mut primary_score = f32::INFINITY;
        let mut primary_index = state.biome_at(center);

        for z in -SURFACE_SITE_SEARCH_RADIUS..=SURFACE_SITE_SEARCH_RADIUS {
            for x in -SURFACE_SITE_SEARCH_RADIUS..=SURFACE_SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                let site = land_site_position(cell, spacing, self.seed);
                let score = warped.distance_squared(site);
                if score < primary_score {
                    primary_cell = cell;
                    primary_site = site;
                    primary_score = score;
                    primary_index = state.biome_at(cell);
                }
            }
        }

        let mut boundary = None;
        for z in -SURFACE_SITE_SEARCH_RADIUS..=SURFACE_SITE_SEARCH_RADIUS {
            for x in -SURFACE_SITE_SEARCH_RADIUS..=SURFACE_SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                if cell == primary_cell {
                    continue;
                }
                let candidate_index = state.biome_at(cell);
                if candidate_index == primary_index {
                    continue;
                }
                let site = land_site_position(cell, spacing, self.seed);
                let pair_distance = primary_site.distance(site);
                if pair_distance <= f32::EPSILON {
                    continue;
                }
                let candidate_score = warped.distance_squared(site);
                let distance =
                    ((candidate_score - primary_score) / (2.0 * pair_distance)).max(0.0);
                let candidate = SurfaceBoundarySample {
                    neighbor_surface_index: candidate_index,
                    neighbor_terrain_strength: 1.0,
                    distance,
                };
                if boundary.is_none_or(|current: SurfaceBoundarySample| {
                    candidate.distance < current.distance
                        || (candidate.distance == current.distance
                            && candidate.neighbor_surface_index < current.neighbor_surface_index)
                }) {
                    boundary = Some(candidate);
                }
            }
        }

        SurfaceFieldSample {
            primary_index,
            primary_terrain_strength: 1.0,
            boundary,
        }
    }

    pub(crate) fn surface_biomes_can_neighbor(&self, left: usize, right: usize) -> bool {
        if left == right {
            return true;
        }

        let left_biome = self
            .surface_biomes
            .get(left)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {left}"));
        let right_biome = self
            .surface_biomes
            .get(right)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {right}"));

        if left_biome.exclusive_neighbor_group.is_some()
            && left_biome.exclusive_neighbor_group == right_biome.exclusive_neighbor_group
        {
            return false;
        }

        !left_biome
            .neighbor_deny
            .as_ref()
            .is_some_and(|selector| selector.matches(&right_biome.id, &right_biome.tags))
            && !right_biome
                .neighbor_deny
                .as_ref()
                .is_some_and(|selector| selector.matches(&left_biome.id, &left_biome.tags))
    }
}

fn ensure_initialized(field: &BiomeField, state: &mut SurfaceBiomeMapState) {
    if state.base_biome_index.is_some() {
        return;
    }

    let base_index = choose_base_biome(field);
    state.base_biome_index = Some(base_index);
    state.placed_counts.resize(field.surface_biomes.len(), 0);
}

fn choose_base_biome(field: &BiomeField) -> usize {
    let mut active = field
        .surface_biomes
        .iter()
        .enumerate()
        .filter(|(index, biome)| biome.weight > 0.0 && field.surface_biome_is_enabled(*index))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    if active.is_empty() {
        active = field
            .surface_biomes
            .iter()
            .enumerate()
            .filter(|(_, biome)| biome.weight > 0.0)
            .map(|(index, _)| index)
            .collect();
    }

    let neutral = active
        .iter()
        .copied()
        .filter(|candidate_index| {
            active.iter().copied().all(|other_index| {
                field.surface_biomes_can_neighbor(*candidate_index, other_index)
            })
        })
        .collect::<Vec<_>>();
    let pool = if neutral.is_empty() { &active } else { &neutral };

    pool.iter()
        .copied()
        .min_by(|left, right| {
            let left_biome = &field.surface_biomes[*left];
            let right_biome = &field.surface_biomes[*right];
            let left_hash = field.seed
                ^ BASE_HASH_SALT
                ^ (*left as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
            let right_hash = field.seed
                ^ BASE_HASH_SALT
                ^ (*right as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
            weighted_random_score(hash_unit(left_hash), left_biome.weight)
                .total_cmp(&weighted_random_score(hash_unit(right_hash), right_biome.weight))
                .then_with(|| left.cmp(right))
        })
        .unwrap_or(0)
}

fn weighted_random_score(unit: f32, weight: f32) -> f32 {
    -unit.max(f32::MIN_POSITIVE).ln() / weight.max(f32::MIN_POSITIVE)
}

fn maximum_stamp_reach_cells(field: &BiomeField) -> i32 {
    field
        .surface_biomes
        .iter()
        .enumerate()
        .filter(|(index, biome)| biome.weight > 0.0 && field.surface_biome_is_enabled(*index))
        .map(|(_, biome)| {
            let (_, max_x) = axis_cell_limits(biome.size.x, field.surface_field_config.land_spacing.x);
            let (_, max_z) = axis_cell_limits(biome.size.z, field.surface_field_config.land_spacing.y);
            (max_x.max(max_z) + 1) / 2
        })
        .max()
        .unwrap_or(1)
}

fn plan_anchor_ring(field: &BiomeField, state: &mut SurfaceBiomeMapState, radius: i32) {
    let mut anchors = ring_cells(radius);
    anchors.sort_unstable_by_key(|anchor| {
        (
            cell_hash(*anchor, field.seed ^ ANCHOR_HASH_SALT),
            anchor.y,
            anchor.x,
        )
    });

    for anchor in anchors {
        let anchor_cell = anchor_cell(anchor, field.seed);
        let sequence = state.attempted_anchors;
        state.attempted_anchors = state.attempted_anchors.saturating_add(1);
        let candidates = ranked_stamp_candidates(field, state, anchor_cell, sequence);
        let _ = try_stamp_candidates(field, state, anchor_cell, sequence, &candidates);
    }
}

fn anchor_cell(anchor: IVec2, seed: u64) -> IVec2 {
    let base = anchor * STAMP_ANCHOR_STRIDE_CELLS;
    if anchor == IVec2::ZERO {
        return base;
    }

    let jitter_x = jitter_component(
        cell_hash(anchor, seed ^ ANCHOR_HASH_SALT ^ 0x243f_6a88_85a3_08d3),
        STAMP_ANCHOR_JITTER_CELLS,
    );
    let jitter_z = jitter_component(
        cell_hash(anchor, seed ^ ANCHOR_HASH_SALT ^ 0x1319_8a2e_0370_7344),
        STAMP_ANCHOR_JITTER_CELLS,
    );
    base + IVec2::new(jitter_x, jitter_z)
}

fn jitter_component(hash: u64, radius: i32) -> i32 {
    if radius <= 0 {
        return 0;
    }
    let width = radius * 2 + 1;
    (hash % width as u64) as i32 - radius
}

fn ranked_stamp_candidates(
    field: &BiomeField,
    state: &SurfaceBiomeMapState,
    anchor: IVec2,
    sequence: u64,
) -> Vec<usize> {
    let base_index = state.base_biome_index.unwrap_or(usize::MAX);
    let mut candidates = field
        .surface_biomes
        .iter()
        .enumerate()
        .filter(|(index, biome)| {
            *index != base_index
                && biome.weight > 0.0
                && field.surface_biome_is_enabled(*index)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    let total_weight = candidates
        .iter()
        .map(|index| field.surface_biomes[*index].weight.max(0.0))
        .sum::<f32>()
        .max(f32::MIN_POSITIVE);
    let target_region_count = state.non_base_region_count().saturating_add(1) as f32;

    candidates.sort_by(|left, right| {
        let left_biome = &field.surface_biomes[*left];
        let right_biome = &field.surface_biomes[*right];
        let left_expected = target_region_count * left_biome.weight / total_weight;
        let right_expected = target_region_count * right_biome.weight / total_weight;
        let left_actual = state.placed_counts.get(*left).copied().unwrap_or(0) as f32;
        let right_actual = state.placed_counts.get(*right).copied().unwrap_or(0) as f32;
        let left_debt = left_expected - left_actual;
        let right_debt = right_expected - right_actual;
        let left_hash = cell_hash(
            anchor,
            field.seed
                ^ SHAPE_HASH_SALT
                ^ sequence.rotate_left(17)
                ^ (*left as u64).wrapping_mul(0x9e37_79b1_85eb_ca87),
        );
        let right_hash = cell_hash(
            anchor,
            field.seed
                ^ SHAPE_HASH_SALT
                ^ sequence.rotate_left(17)
                ^ (*right as u64).wrapping_mul(0x9e37_79b1_85eb_ca87),
        );

        right_debt
            .total_cmp(&left_debt)
            .then_with(|| left_hash.cmp(&right_hash))
            .then_with(|| left.cmp(right))
    });

    candidates.truncate(MAX_BIOME_CANDIDATES_PER_ANCHOR);
    candidates
}

fn try_stamp_candidates(
    field: &BiomeField,
    state: &mut SurfaceBiomeMapState,
    anchor: IVec2,
    sequence: u64,
    candidates: &[usize],
) -> bool {
    for (level_index, irregularity) in SHAPE_IRREGULARITY_LEVELS.into_iter().enumerate() {
        for &biome_index in candidates {
            let attempt_seed = field.seed
                ^ SHAPE_HASH_SALT
                ^ (biome_index as u64).wrapping_mul(0x9e37_79b1_85eb_ca87)
                ^ (level_index as u64).rotate_left(13)
                ^ sequence.rotate_left(7);
            let shape_seed = cell_hash(anchor, attempt_seed);
            let offset = placement_offset(
                anchor,
                attempt_seed
                    ^ (biome_index as u64).wrapping_mul(0xd6e8_feb8_6659_fd93)
                    ^ sequence.rotate_left(29),
            );
            let center = anchor + offset;
            let Some(shape) = generate_stamp_shape(
                field,
                biome_index,
                center,
                shape_seed,
                irregularity,
            ) else {
                continue;
            };
            if !stamp_placement_is_valid(field, state, biome_index, &shape) {
                continue;
            }

            apply_region(state, biome_index, &shape);
            return true;
        }
    }

    false
}

fn placement_offset(anchor: IVec2, seed: u64) -> IVec2 {
    let slot = (cell_hash(anchor, seed) % 9) as i32;
    let x = (slot % 3 - 1) * STAMP_POSITION_OFFSET_CELLS;
    let z = (slot / 3 - 1) * STAMP_POSITION_OFFSET_CELLS;
    IVec2::new(x, z)
}

fn generate_stamp_shape(
    field: &BiomeField,
    biome_index: usize,
    center: IVec2,
    shape_seed: u64,
    irregularity: f32,
) -> Option<StampShape> {
    let biome = &field.surface_biomes[biome_index];
    let spacing = field.surface_field_config.land_spacing;
    let (min_x, max_x) = axis_cell_limits(biome.size.x, spacing.x);
    let (min_z, max_z) = axis_cell_limits(biome.size.z, spacing.y);
    let span_x = choose_span(min_x, max_x, hash_unit(shape_seed.rotate_left(11)));
    let span_z = choose_span(min_z, max_z, hash_unit(shape_seed.rotate_left(37)));

    let local_min_x = -(span_x / 2);
    let local_min_z = -(span_z / 2);
    let local_max_x = local_min_x + span_x - 1;
    let local_max_z = local_min_z + span_z - 1;
    let mut membership = HashSet::new();

    for local_z in local_min_z..=local_max_z {
        for local_x in local_min_x..=local_max_x {
            let nx = (((local_x - local_min_x) as f32 + 0.5) / span_x as f32) * 2.0 - 1.0;
            let nz = (((local_z - local_min_z) as f32 + 0.5) / span_z as f32) * 2.0 - 1.0;
            let radial = Vec2::new(nx, nz).length();
            let angle = nz.atan2(nx);
            let boundary = 1.0 - irregularity * angular_indent(angle, shape_seed);
            if radial <= boundary {
                membership.insert(center + IVec2::new(local_x, local_z));
            }
        }
    }

    if membership.is_empty() || !shape_is_connected(&membership, center) {
        return None;
    }

    let (minimum, maximum) = bounds_of_cells(membership.iter().copied())?;
    let span = maximum - minimum + IVec2::ONE;
    if span.x < min_x || span.x > max_x || span.y < min_z || span.y > max_z {
        return None;
    }

    let mut cells = membership.iter().copied().collect::<Vec<_>>();
    cells.sort_unstable_by_key(|cell| (cell.y, cell.x));
    Some(StampShape {
        cells,
        membership,
        minimum,
        maximum,
    })
}

fn angular_indent(angle: f32, seed: u64) -> f32 {
    let sector_position = ((angle + std::f32::consts::PI) / std::f32::consts::TAU)
        * SHAPE_ANGLE_SECTORS as f32;
    let sector = sector_position.floor() as i32;
    let fraction = sector_position - sector as f32;
    let smooth = fraction * fraction * (3.0 - 2.0 * fraction);
    let first = hash_unit(cell_hash(
        IVec2::new(sector.rem_euclid(SHAPE_ANGLE_SECTORS), 0),
        seed,
    ));
    let second = hash_unit(cell_hash(
        IVec2::new((sector + 1).rem_euclid(SHAPE_ANGLE_SECTORS), 0),
        seed,
    ));
    let noise = first + (second - first) * smooth;
    0.15 + noise * 0.85
}

fn shape_is_connected(cells: &HashSet<IVec2>, center: IVec2) -> bool {
    let Some(start) = cells
        .iter()
        .copied()
        .min_by_key(|cell| ((*cell - center).length_squared(), cell.y, cell.x))
    else {
        return false;
    };

    let mut seen = HashSet::new();
    let mut stack = vec![start];
    while let Some(cell) = stack.pop() {
        if !seen.insert(cell) {
            continue;
        }
        for neighbor in cardinal_neighbors(cell) {
            if cells.contains(&neighbor) && !seen.contains(&neighbor) {
                stack.push(neighbor);
            }
        }
    }
    seen.len() == cells.len()
}

fn stamp_placement_is_valid(
    field: &BiomeField,
    state: &SurfaceBiomeMapState,
    biome_index: usize,
    shape: &StampShape,
) -> bool {
    if shape.cells.iter().any(|cell| state.cells.contains_key(cell)) {
        return false;
    }

    let base_index = state.base_biome_index.unwrap_or(biome_index);
    let mut touches_non_base_region = false;

    for &cell in &shape.cells {
        for neighbor in cardinal_neighbors(cell) {
            if shape.membership.contains(&neighbor) {
                continue;
            }
            let neighbor_index = state
                .cells
                .get(&neighbor)
                .map(|resolved| resolved.biome_index)
                .unwrap_or(base_index);

            if neighbor_index == biome_index {
                return false;
            }
            if !field.surface_biomes_can_neighbor(biome_index, neighbor_index) {
                return false;
            }
            if neighbor_index != base_index {
                touches_non_base_region = true;
            }
        }
    }

    if !touches_non_base_region {
        return true;
    }

    base_components_remain_valid(field, state, shape)
}

fn base_components_remain_valid(
    field: &BiomeField,
    state: &SurfaceBiomeMapState,
    candidate: &StampShape,
) -> bool {
    let Some(base_index) = state.base_biome_index else {
        return true;
    };
    let base = &field.surface_biomes[base_index];
    let spacing = field.surface_field_config.land_spacing;
    let (min_x, max_x) = axis_cell_limits(base.size.x, spacing.x);
    let (min_z, max_z) = axis_cell_limits(base.size.z, spacing.y);

    let painted_minimum = state
        .painted_minimum
        .map_or(candidate.minimum, |current| current.min(candidate.minimum));
    let painted_maximum = state
        .painted_maximum
        .map_or(candidate.maximum, |current| current.max(candidate.maximum));
    let outside_minimum = painted_minimum - IVec2::ONE;
    let outside_maximum = painted_maximum + IVec2::ONE;

    let mut known_open = HashSet::new();
    let mut known_finite = HashSet::new();
    let mut starts = Vec::new();
    for &cell in &candidate.cells {
        for neighbor in cardinal_neighbors(cell) {
            if state.is_simulated_base_cell(neighbor, &candidate.membership)
                && !starts.contains(&neighbor)
            {
                starts.push(neighbor);
            }
        }
    }
    starts.sort_unstable_by_key(|cell| (cell.y, cell.x));

    for start in starts {
        if known_open.contains(&start) || known_finite.contains(&start) {
            continue;
        }
        let component = classify_base_component(
            state,
            &candidate.membership,
            start,
            outside_minimum,
            outside_maximum,
            &known_open,
        );
        match component {
            BaseComponent::Open { visited } => {
                known_open.extend(visited);
            }
            BaseComponent::Finite {
                visited,
                minimum,
                maximum,
            } => {
                let span = maximum - minimum + IVec2::ONE;
                if span.x < min_x || span.x > max_x || span.y < min_z || span.y > max_z {
                    return false;
                }
                known_finite.extend(visited);
            }
        }
    }

    true
}

enum BaseComponent {
    Open { visited: Vec<IVec2> },
    Finite {
        visited: Vec<IVec2>,
        minimum: IVec2,
        maximum: IVec2,
    },
}

fn classify_base_component(
    state: &SurfaceBiomeMapState,
    candidate: &HashSet<IVec2>,
    start: IVec2,
    outside_minimum: IVec2,
    outside_maximum: IVec2,
    known_open: &HashSet<IVec2>,
) -> BaseComponent {
    let mut local_seen = HashSet::new();
    let mut visited = Vec::new();
    let mut stack = vec![start];
    let mut minimum = start;
    let mut maximum = start;

    while let Some(cell) = stack.pop() {
        if known_open.contains(&cell)
            || cell.x < outside_minimum.x
            || cell.x > outside_maximum.x
            || cell.y < outside_minimum.y
            || cell.y > outside_maximum.y
        {
            return BaseComponent::Open { visited };
        }
        if !state.is_simulated_base_cell(cell, candidate) || !local_seen.insert(cell) {
            continue;
        }

        visited.push(cell);
        minimum = minimum.min(cell);
        maximum = maximum.max(cell);
        for neighbor in cardinal_neighbors(cell) {
            if known_open.contains(&neighbor)
                || neighbor.x < outside_minimum.x
                || neighbor.x > outside_maximum.x
                || neighbor.y < outside_minimum.y
                || neighbor.y > outside_maximum.y
            {
                return BaseComponent::Open { visited };
            }
            if state.is_simulated_base_cell(neighbor, candidate)
                && !local_seen.contains(&neighbor)
            {
                stack.push(neighbor);
            }
        }
    }

    BaseComponent::Finite {
        visited,
        minimum,
        maximum,
    }
}

fn apply_region(state: &mut SurfaceBiomeMapState, biome_index: usize, shape: &StampShape) {
    for &cell in &shape.cells {
        state.cells.insert(cell, SurfaceBiomeCell { biome_index });
    }
    if let Some(count) = state.placed_counts.get_mut(biome_index) {
        *count = count.saturating_add(1);
    }

    if Some(biome_index) != state.base_biome_index {
        state.painted_minimum = Some(
            state
                .painted_minimum
                .map_or(shape.minimum, |current| current.min(shape.minimum)),
        );
        state.painted_maximum = Some(
            state
                .painted_maximum
                .map_or(shape.maximum, |current| current.max(shape.maximum)),
        );
    }
}

fn bounds_of_cells(cells: impl Iterator<Item = IVec2>) -> Option<(IVec2, IVec2)> {
    let mut cells = cells;
    let first = cells.next()?;
    let mut minimum = first;
    let mut maximum = first;
    for cell in cells {
        minimum = minimum.min(cell);
        maximum = maximum.max(cell);
    }
    Some((minimum, maximum))
}

fn axis_cell_limits(size: DimensionBiomeSizeAxis, spacing: f32) -> (i32, i32) {
    let minimum = (size.min / spacing).ceil().max(1.0) as i32;
    let maximum = (size.max / spacing)
        .floor()
        .max(minimum as f32) as i32;
    (minimum, maximum)
}

fn choose_span(minimum: i32, maximum: i32, unit: f32) -> i32 {
    if minimum >= maximum {
        return minimum;
    }
    let range = maximum - minimum + 1;
    minimum + ((unit * range as f32).floor() as i32).min(range - 1)
}

fn cardinal_neighbors(cell: IVec2) -> [IVec2; 4] {
    [
        cell + IVec2::X,
        cell - IVec2::X,
        cell + IVec2::Y,
        cell - IVec2::Y,
    ]
}

fn ring_cells(radius: i32) -> Vec<IVec2> {
    if radius == 0 {
        return vec![IVec2::ZERO];
    }

    let mut cells = Vec::with_capacity((radius * 8) as usize);
    for x in -radius..=radius {
        cells.push(IVec2::new(x, -radius));
        cells.push(IVec2::new(x, radius));
    }
    for z in (-radius + 1)..=(radius - 1) {
        cells.push(IVec2::new(-radius, z));
        cells.push(IVec2::new(radius, z));
    }
    cells
}

fn div_ceil_i32(value: i32, divisor: i32) -> i32 {
    if value <= 0 {
        return 0;
    }
    (value + divisor - 1) / divisor
}

pub(super) fn land_site_position(cell: IVec2, spacing: Vec2, seed: u64) -> Vec2 {
    let base = Vec2::new(cell.x as f32 * spacing.x, cell.y as f32 * spacing.y);
    if cell.y == 0 {
        return base;
    }

    let row_hash = cell_hash(
        IVec2::new(0, cell.y),
        seed ^ 0xd1b5_4a32_d192_ed03,
    );
    let row_offset = hash_signed(row_hash) * spacing.x * SURFACE_ROW_JITTER_FRACTION;
    base + Vec2::new(row_offset, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_size_is_converted_to_minimum_and_maximum_cell_span() {
        let axis = DimensionBiomeSizeAxis {
            min: 80.0,
            max: 180.0,
        };
        assert_eq!(axis_cell_limits(axis, SURFACE_MAP_CELL_SIZE), (10, 22));
    }

    #[test]
    fn row_jitter_preserves_horizontal_site_spacing() {
        let spacing = Vec2::splat(SURFACE_MAP_CELL_SIZE);
        let seed = 42;
        for y in -4..=4 {
            for x in -4..=4 {
                let cell = IVec2::new(x, y);
                let site = land_site_position(cell, spacing, seed);
                let right = land_site_position(cell + IVec2::X, spacing, seed);
                assert!((right.x - site.x - spacing.x).abs() < 1e-5);
                assert!((right.y - site.y).abs() < 1e-5);
            }
        }
    }
}
