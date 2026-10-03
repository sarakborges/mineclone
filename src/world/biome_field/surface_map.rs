use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{content::dimension::DimensionBiomeSizeAxis, voxel::chunk::CHUNK_SIZE};

use super::{
    BiomeField, SurfaceSiteCacheEntry,
    constants::SITE_SEARCH_RADIUS,
    selection::{surface_biomes_conflict, surface_requirement_satisfied},
    spatial::{cell_hash, hash_unit, surface_site_position},
};

const BASE_HASH_SALT: u64 = 0x6a09_e667_f3bc_c909;
const ANCHOR_HASH_SALT: u64 = 0xbb67_ae85_84ca_a73b;
const SHAPE_HASH_SALT: u64 = 0x3c6e_f372_fe94_f82b;
const STAMP_ANCHOR_STRIDE_CELLS: i32 = 8;
const STAMP_ANCHOR_JITTER_CELLS: i32 = 2;
const STAMP_POSITION_OFFSET_CELLS: i32 = 2;
const SHAPE_VARIANTS_PER_LEVEL: u64 = 2;
const SHAPE_ANGLE_SECTORS: i32 = 24;
const SHAPE_IRREGULARITY_LEVELS: [f32; 5] = [0.34, 0.24, 0.14, 0.07, 0.0];

#[derive(Clone)]
pub(crate) struct SurfaceBiomeMap {
    state: Arc<RwLock<SurfaceBiomeMapState>>,
    samples: Arc<RwLock<HashMap<IVec2, SurfaceSiteCacheEntry>>>,
}

impl Default for SurfaceBiomeMap {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(SurfaceBiomeMapState::default())),
            samples: Arc::new(RwLock::new(HashMap::new())),
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

impl SurfaceBiomeMap {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn ensure_sample_window(&self, field: &BiomeField, center: IVec2) {
        let required_radius = center
            .x
            .abs()
            .max(center.y.abs())
            .saturating_add(SITE_SEARCH_RADIUS);
        self.ensure_planned_through(field, required_radius);

        let state = self
            .state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut samples = self
            .samples
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                samples.entry(cell).or_insert_with(|| SurfaceSiteCacheEntry {
                    position: surface_site_position(cell, field.surface_site_spacing, field.seed),
                    biome_index: state.biome_at(cell),
                });
            }
        }
    }

    pub(super) fn read_samples(
        &self,
    ) -> LockResult<RwLockReadGuard<'_, HashMap<IVec2, SurfaceSiteCacheEntry>>> {
        self.samples.read()
    }

    pub(super) fn clear(&self) {
        *self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = SurfaceBiomeMapState::default();
        self.samples
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    pub(super) fn retain_samples_around(
        &self,
        center_chunk: IVec2,
        radius_chunks: i32,
        surface_site_spacing: Vec2,
    ) {
        let chunk_size = CHUNK_SIZE as i32;
        let center_world = (center_chunk * chunk_size).as_vec2()
            + Vec2::splat(CHUNK_SIZE as f32 * 0.5);
        let center_cell = IVec2::new(
            (center_world.x / surface_site_spacing.x).round() as i32,
            (center_world.y / surface_site_spacing.y).round() as i32,
        );
        let world_radius = radius_chunks
            .max(0)
            .saturating_add(2)
            .saturating_mul(chunk_size) as f32;
        let padding = SITE_SEARCH_RADIUS + 2;
        let radius_x = (world_radius / surface_site_spacing.x).ceil() as i32 + padding;
        let radius_z = (world_radius / surface_site_spacing.y).ceil() as i32 + padding;

        self.samples
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|cell, _| {
                (cell.x - center_cell.x).abs() <= radius_x
                    && (cell.y - center_cell.y).abs() <= radius_z
            });
    }

    fn ensure_planned_through(&self, field: &BiomeField, required_cell_radius: i32) {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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

fn ensure_initialized(field: &BiomeField, state: &mut SurfaceBiomeMapState) {
    if state.base_biome_index.is_some() {
        return;
    }

    let base_index = choose_base_biome(field);
    state.base_biome_index = Some(base_index);
    state.placed_counts.resize(field.surface_biomes.len(), 0);

    let source_hash = cell_hash(IVec2::ZERO, field.seed ^ BASE_HASH_SALT);
    for (level_index, irregularity) in SHAPE_IRREGULARITY_LEVELS.into_iter().enumerate() {
        for variant in 0..SHAPE_VARIANTS_PER_LEVEL {
            let shape_seed = source_hash
                ^ (level_index as u64).wrapping_mul(0x9e37_79b1_85eb_ca87)
                ^ variant.wrapping_mul(0xd6e8_feb8_6659_fd93);
            if let Some(shape) = generate_stamp_shape(
                field,
                base_index,
                IVec2::ZERO,
                shape_seed,
                irregularity,
            ) {
                apply_region(state, base_index, &shape);
                return;
            }
        }
    }
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
            let candidate = &field.surface_biomes[*candidate_index];
            candidate.require_near.is_empty()
                && active.iter().copied().all(|other_index| {
                    !surface_biomes_conflict(candidate, &field.surface_biomes[other_index])
                })
        })
        .collect::<Vec<_>>();

    let self_sufficient = active
        .iter()
        .copied()
        .filter(|index| field.surface_biomes[*index].require_near.is_empty())
        .collect::<Vec<_>>();

    let pool = if !neutral.is_empty() {
        &neutral
    } else if !self_sufficient.is_empty() {
        &self_sufficient
    } else {
        &active
    };

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
            let (_, max_x) = axis_cell_limits(biome.size.x, field.surface_site_spacing.x);
            let (_, max_z) = axis_cell_limits(biome.size.z, field.surface_site_spacing.y);
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
        if try_stamp_candidates(field, state, anchor_cell, sequence, &candidates) {
            continue;
        }
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

    candidates
}

fn try_stamp_candidates(
    field: &BiomeField,
    state: &mut SurfaceBiomeMapState,
    anchor: IVec2,
    sequence: u64,
    candidates: &[usize],
) -> bool {
    for &biome_index in candidates {
        let offsets = placement_offsets(
            anchor,
            field.seed
                ^ (biome_index as u64).wrapping_mul(0xd6e8_feb8_6659_fd93)
                ^ sequence.rotate_left(29),
        );

        for (level_index, irregularity) in SHAPE_IRREGULARITY_LEVELS.into_iter().enumerate() {
            for variant in 0..SHAPE_VARIANTS_PER_LEVEL {
                let shape_seed = cell_hash(
                    anchor,
                    field.seed
                        ^ SHAPE_HASH_SALT
                        ^ (biome_index as u64).wrapping_mul(0x9e37_79b1_85eb_ca87)
                        ^ (level_index as u64).rotate_left(13)
                        ^ variant.wrapping_mul(0xa24b_aed4_963e_e407)
                        ^ sequence.rotate_left(7),
                );

                for &offset in &offsets {
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
        }
    }

    false
}

fn placement_offsets(anchor: IVec2, seed: u64) -> Vec<IVec2> {
    let d = STAMP_POSITION_OFFSET_CELLS;
    let mut offsets = Vec::with_capacity(9);
    for z in [-d, 0, d] {
        for x in [-d, 0, d] {
            offsets.push(IVec2::new(x, z));
        }
    }
    offsets.sort_unstable_by_key(|offset| {
        let cell = anchor + *offset;
        (cell_hash(cell, seed), offset.y, offset.x)
    });
    offsets
}

fn generate_stamp_shape(
    field: &BiomeField,
    biome_index: usize,
    center: IVec2,
    shape_seed: u64,
    irregularity: f32,
) -> Option<StampShape> {
    let biome = &field.surface_biomes[biome_index];
    let (min_x, max_x) = axis_cell_limits(biome.size.x, field.surface_site_spacing.x);
    let (min_z, max_z) = axis_cell_limits(biome.size.z, field.surface_site_spacing.y);
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

    let candidate = &field.surface_biomes[biome_index];
    let base_index = state.base_biome_index.unwrap_or(biome_index);
    let mut neighbor_indices = Vec::new();

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
            if surface_biomes_conflict(candidate, &field.surface_biomes[neighbor_index]) {
                return false;
            }
            if !neighbor_indices.contains(&neighbor_index) {
                neighbor_indices.push(neighbor_index);
            }
        }
    }

    if !surface_requirement_satisfied(candidate, &neighbor_indices, &field.surface_biomes) {
        return false;
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
    let (min_x, max_x) = axis_cell_limits(base.size.x, field.surface_site_spacing.x);
    let (min_z, max_z) = axis_cell_limits(base.size.z, field.surface_site_spacing.y);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        content::{
            biome::BiomeClimate,
            biome_distribution::BiomeDistribution,
            dimension::DimensionBiomeSize,
        },
        world::macro_climate::MacroClimateField,
    };

    fn test_size(min: f32, max: f32) -> DimensionBiomeSize {
        let axis = DimensionBiomeSizeAxis { min, max };
        DimensionBiomeSize {
            x: axis,
            z: axis,
            y: None,
        }
    }

    fn test_entry(
        id: &str,
        weight: f32,
        size: DimensionBiomeSize,
        exclusive_neighbor_group: Option<&str>,
    ) -> super::super::BiomeFieldEntry {
        super::super::BiomeFieldEntry {
            id: id.to_owned(),
            tags: Vec::new(),
            surface_constraints: None,
            distributions: vec![BiomeDistribution::Regional],
            size,
            weight,
            climate: BiomeClimate::default(),
            vertical_range: None,
            priority: 0,
            terrain: None,
            terrain_modifiers: Vec::new(),
            density_modifier: None,
            solid_block: None,
            density_seed: 0,
            avoid_near: Vec::new(),
            require_near: Vec::new(),
            exclusive_neighbor_group: exclusive_neighbor_group.map(str::to_owned),
            surface_margin: None,
        }
    }

    fn test_field() -> BiomeField {
        let size = test_size(48.0, 96.0);
        BiomeField {
            surface_biomes: Arc::new(vec![
                test_entry("test:base_a", 1.0, size, None),
                test_entry("test:base_b", 1.0, size, None),
                test_entry("test:exclusive", 0.7, size, Some("exclusive")),
            ]),
            volume_biomes: Arc::new(Vec::new()),
            surface_site_spacing: Vec2::splat(8.0),
            volume_site_spacing: None,
            climate: MacroClimateField::new(42),
            seed: 42,
            surface_map: SurfaceBiomeMap::new(),
            forced_surface_biome: None,
            single_surface_biome: None,
            ocean_surface_index: None,
            spawn_oceans: true,
        }
    }

    #[test]
    fn authored_surface_size_is_interpreted_as_diameter() {
        let axis = DimensionBiomeSizeAxis {
            min: 80.0,
            max: 180.0,
        };
        assert_eq!(axis_cell_limits(axis, 8.0), (10, 22));
    }

    #[test]
    fn generated_stamp_is_connected_and_within_authored_size() {
        let field = test_field();
        let shape = generate_stamp_shape(&field, 0, IVec2::ZERO, 17, 0.24)
            .or_else(|| generate_stamp_shape(&field, 0, IVec2::ZERO, 17, 0.0))
            .expect("fallback stamp must be constructible");
        let (min_x, max_x) = axis_cell_limits(field.surface_biomes[0].size.x, 8.0);
        let (min_z, max_z) = axis_cell_limits(field.surface_biomes[0].size.z, 8.0);
        let span = shape.maximum - shape.minimum + IVec2::ONE;
        assert!((min_x..=max_x).contains(&span.x));
        assert!((min_z..=max_z).contains(&span.y));
        assert!(shape_is_connected(&shape.membership, IVec2::ZERO));
    }

    #[test]
    fn base_selection_avoids_biomes_that_block_the_pool() {
        let field = test_field();
        let base = choose_base_biome(&field);
        assert_ne!(base, 2);
    }

    #[test]
    fn extending_the_plan_never_changes_already_planned_cells() {
        let field = test_field();
        field.surface_map.ensure_planned_through(&field, 10);
        let before = {
            let state = field
                .surface_map
                .state
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (-10..=10)
                .flat_map(|z| (-10..=10).map(move |x| IVec2::new(x, z)))
                .map(|cell| (cell, state.biome_at(cell)))
                .collect::<Vec<_>>()
        };

        field.surface_map.ensure_planned_through(&field, 40);
        let state = field
            .surface_map
            .state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(before
            .into_iter()
            .all(|(cell, biome_index)| state.biome_at(cell) == biome_index));
    }

    #[test]
    fn accepted_stamps_never_overwrite_existing_regions() {
        let field = test_field();
        let mut state = SurfaceBiomeMapState::default();
        ensure_initialized(&field, &mut state);
        let initial = state.cells.clone();

        plan_anchor_ring(&field, &mut state, 1);

        assert!(initial.iter().all(|(cell, previous)| {
            state
                .cells
                .get(cell)
                .is_some_and(|current| current.biome_index == previous.biome_index)
        }));
    }
}
