use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard};

use bevy::{platform::collections::HashMap, prelude::*};

use crate::{content::dimension::DimensionBiomeSizeAxis, voxel::chunk::CHUNK_SIZE};

use super::{
    BiomeField, SurfaceSiteCacheEntry,
    constants::SITE_SEARCH_RADIUS,
    selection::{surface_biomes_conflict, surface_requirement_satisfied},
    spatial::{cell_hash, hash_unit, surface_site_position},
};

const REGION_HASH_SALT: u64 = 0x94d0_49bb_1331_11eb;
const TARGET_X_HASH_SALT: u64 = 0x517c_c1b7_2722_0a95;
const TARGET_Z_HASH_SALT: u64 = 0x9e37_79b9_7f4a_7c15;

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
    region_id: usize,
}

#[derive(Debug)]
struct SurfaceBiomeRegion {
    biome_index: usize,
    min_span: IVec2,
    target_span: IVec2,
    max_span: IVec2,
    minimum: IVec2,
    maximum: IVec2,
}

impl SurfaceBiomeRegion {
    fn new(
        biome_index: usize,
        origin: IVec2,
        size_x: DimensionBiomeSizeAxis,
        size_z: DimensionBiomeSizeAxis,
        spacing: Vec2,
        source_hash: u64,
    ) -> Self {
        let (min_x, max_x) = axis_cell_limits(size_x, spacing.x);
        let (min_z, max_z) = axis_cell_limits(size_z, spacing.y);
        let target_x = choose_target_span(min_x, max_x, hash_unit(source_hash ^ TARGET_X_HASH_SALT));
        let target_z = choose_target_span(min_z, max_z, hash_unit(source_hash ^ TARGET_Z_HASH_SALT));

        Self {
            biome_index,
            min_span: IVec2::new(min_x, min_z),
            target_span: IVec2::new(target_x, target_z),
            max_span: IVec2::new(max_x, max_z),
            minimum: origin,
            maximum: origin,
        }
    }

    fn span(&self) -> IVec2 {
        self.maximum - self.minimum + IVec2::ONE
    }

    fn proposed_bounds(&self, cell: IVec2) -> (IVec2, IVec2, IVec2) {
        let minimum = self.minimum.min(cell);
        let maximum = self.maximum.max(cell);
        let span = maximum - minimum + IVec2::ONE;
        (minimum, maximum, span)
    }

    fn can_claim(&self, cell: IVec2) -> bool {
        let (_, _, span) = self.proposed_bounds(cell);
        span.x <= self.max_span.x && span.y <= self.max_span.y
    }

    fn claim(&mut self, cell: IVec2) {
        self.minimum = self.minimum.min(cell);
        self.maximum = self.maximum.max(cell);
    }

    fn needs_minimum(&self) -> bool {
        let span = self.span();
        span.x < self.min_span.x || span.y < self.min_span.y
    }

    fn needs_target(&self) -> bool {
        let span = self.span();
        span.x < self.target_span.x || span.y < self.target_span.y
    }

    fn improves_minimum(&self, cell: IVec2) -> bool {
        improves_deficient_axis(self.span(), self.proposed_bounds(cell).2, self.min_span)
    }

    fn improves_target(&self, cell: IVec2) -> bool {
        improves_deficient_axis(self.span(), self.proposed_bounds(cell).2, self.target_span)
    }

    fn minimum_deficit(&self) -> i32 {
        span_deficit(self.span(), self.min_span)
    }

    fn target_deficit(&self) -> i32 {
        span_deficit(self.span(), self.target_span)
    }

    fn remaining_capacity(&self) -> i32 {
        span_deficit(self.span(), self.max_span)
    }
}

struct SurfaceBiomeMapState {
    resolved_radius: i32,
    cells: HashMap<IVec2, SurfaceBiomeCell>,
    regions: Vec<SurfaceBiomeRegion>,
}

impl Default for SurfaceBiomeMapState {
    fn default() -> Self {
        Self {
            resolved_radius: -1,
            cells: HashMap::new(),
            regions: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
enum GrowthMode {
    Minimum,
    Target,
    Maximum,
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
        self.ensure_resolved_through(field, required_radius);

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
                samples.entry(cell).or_insert_with(|| {
                    let resolved = state.cells.get(&cell).unwrap_or_else(|| {
                        panic!("surface biome map did not resolve requested cell {cell:?}")
                    });
                    SurfaceSiteCacheEntry {
                        position: surface_site_position(cell, field.surface_site_spacing, field.seed),
                        biome_index: resolved.biome_index,
                    }
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

    fn ensure_resolved_through(&self, field: &BiomeField, required_radius: i32) {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        while state.resolved_radius < required_radius {
            let next_radius = state.resolved_radius + 1;
            expand_ring(field, &mut state, next_radius);
            state.resolved_radius = next_radius;
        }
    }
}

fn expand_ring(field: &BiomeField, state: &mut SurfaceBiomeMapState, radius: i32) {
    let mut remaining = ring_cells(radius);
    remaining.sort_unstable_by_key(|cell| {
        (
            cell_hash(*cell, field.seed ^ REGION_HASH_SALT),
            cell.y,
            cell.x,
        )
    });

    while !remaining.is_empty() {
        if claim_existing(field, state, &remaining, GrowthMode::Minimum) {
            remaining.retain(|cell| !state.cells.contains_key(cell));
            continue;
        }

        if claim_existing(field, state, &remaining, GrowthMode::Target) {
            remaining.retain(|cell| !state.cells.contains_key(cell));
            continue;
        }

        if spawn_region(field, state, &remaining) {
            remaining.retain(|cell| !state.cells.contains_key(cell));
            continue;
        }

        if claim_existing(field, state, &remaining, GrowthMode::Maximum) {
            remaining.retain(|cell| !state.cells.contains_key(cell));
            continue;
        }

        let cell = remaining[0];
        panic!(
            "surface biome frontier at radius {radius} cannot resolve cell {cell:?} without exceeding max size or violating authored adjacency constraints"
        );
    }
}

fn claim_existing(
    field: &BiomeField,
    state: &mut SurfaceBiomeMapState,
    remaining: &[IVec2],
    mode: GrowthMode,
) -> bool {
    let mut changed = false;

    for &cell in remaining {
        if state.cells.contains_key(&cell) {
            continue;
        }
        let Some(region_id) = best_neighbor_region(field, state, cell, mode) else {
            continue;
        };

        let biome_index = state.regions[region_id].biome_index;
        state.regions[region_id].claim(cell);
        state.cells.insert(
            cell,
            SurfaceBiomeCell {
                biome_index,
                region_id,
            },
        );
        changed = true;
    }

    changed
}

fn best_neighbor_region(
    field: &BiomeField,
    state: &SurfaceBiomeMapState,
    cell: IVec2,
    mode: GrowthMode,
) -> Option<usize> {
    let mut candidates = [usize::MAX; 4];
    let mut candidate_count = 0;

    for neighbor in cardinal_neighbors(cell) {
        let Some(neighbor_cell) = state.cells.get(&neighbor) else {
            continue;
        };
        if candidates[..candidate_count].contains(&neighbor_cell.region_id) {
            continue;
        }
        candidates[candidate_count] = neighbor_cell.region_id;
        candidate_count += 1;
    }

    let mut best: Option<(i32, u64, usize)> = None;
    for &region_id in &candidates[..candidate_count] {
        let region = &state.regions[region_id];
        if !region.can_claim(cell) || !claim_respects_adjacency(field, state, region_id, cell) {
            continue;
        }

        let deficit = match mode {
            GrowthMode::Minimum => {
                if !region.needs_minimum() || !region.improves_minimum(cell) {
                    continue;
                }
                region.minimum_deficit()
            }
            GrowthMode::Target => {
                if region.needs_minimum()
                    || !region.needs_target()
                    || !region.improves_target(cell)
                {
                    continue;
                }
                region.target_deficit()
            }
            GrowthMode::Maximum => region.remaining_capacity(),
        };
        let claim_hash = cell_hash(
            cell,
            field.seed
                ^ REGION_HASH_SALT
                ^ (region_id as u64).wrapping_mul(0x9e37_79b1_85eb_ca87),
        );
        let candidate = (deficit, claim_hash, region_id);
        if best.is_none_or(|current| {
            candidate.0 > current.0
                || (candidate.0 == current.0
                    && (candidate.1 < current.1
                        || (candidate.1 == current.1 && candidate.2 < current.2)))
        }) {
            best = Some(candidate);
        }
    }

    best.map(|(_, _, region_id)| region_id)
}

fn spawn_region(field: &BiomeField, state: &mut SurfaceBiomeMapState, remaining: &[IVec2]) -> bool {
    for &cell in remaining {
        if state.cells.contains_key(&cell) {
            continue;
        }

        let neighbor_indices = neighbor_biome_indices(state, cell);
        let position = surface_site_position(cell, field.surface_site_spacing, field.seed);
        let candidates = field.ranked_surface_biome_indices(cell, position);

        for biome_index in candidates {
            let biome = &field.surface_biomes[biome_index];
            if neighbor_indices.iter().copied().any(|neighbor_index| {
                surface_biomes_conflict(biome, &field.surface_biomes[neighbor_index])
            }) {
                continue;
            }
            if !surface_requirement_satisfied(biome, &neighbor_indices, &field.surface_biomes) {
                continue;
            }

            let region_id = state.regions.len();
            let source_hash = cell_hash(
                cell,
                field.seed
                    ^ REGION_HASH_SALT
                    ^ (biome_index as u64).wrapping_mul(0xd6e8_feb8_6659_fd93),
            );
            state.regions.push(SurfaceBiomeRegion::new(
                biome_index,
                cell,
                biome.size.x,
                biome.size.z,
                field.surface_site_spacing,
                source_hash,
            ));
            state.cells.insert(
                cell,
                SurfaceBiomeCell {
                    biome_index,
                    region_id,
                },
            );
            return true;
        }
    }

    false
}

fn claim_respects_adjacency(
    field: &BiomeField,
    state: &SurfaceBiomeMapState,
    region_id: usize,
    cell: IVec2,
) -> bool {
    let region = &state.regions[region_id];
    let biome = &field.surface_biomes[region.biome_index];

    cardinal_neighbors(cell).into_iter().all(|neighbor| {
        let Some(neighbor_cell) = state.cells.get(&neighbor) else {
            return true;
        };
        if neighbor_cell.region_id == region_id {
            return true;
        }

        !surface_biomes_conflict(biome, &field.surface_biomes[neighbor_cell.biome_index])
    })
}

fn neighbor_biome_indices(state: &SurfaceBiomeMapState, cell: IVec2) -> Vec<usize> {
    let mut indices = Vec::with_capacity(4);
    for neighbor in cardinal_neighbors(cell) {
        let Some(neighbor_cell) = state.cells.get(&neighbor) else {
            continue;
        };
        if !indices.contains(&neighbor_cell.biome_index) {
            indices.push(neighbor_cell.biome_index);
        }
    }
    indices
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

fn axis_cell_limits(size: DimensionBiomeSizeAxis, spacing: f32) -> (i32, i32) {
    // Surface size values are authored radii. The frontier map tracks the full
    // occupied span, so each axis uses twice the authored radius.
    let minimum = ((size.min * 2.0) / spacing).ceil().max(1.0) as i32;
    let maximum = ((size.max * 2.0) / spacing)
        .floor()
        .max(minimum as f32) as i32;
    (minimum, maximum)
}

fn choose_target_span(minimum: i32, maximum: i32, unit: f32) -> i32 {
    if minimum >= maximum {
        return minimum;
    }
    let range = maximum - minimum + 1;
    minimum + ((unit * range as f32).floor() as i32).min(range - 1)
}

fn improves_deficient_axis(current: IVec2, proposed: IVec2, goal: IVec2) -> bool {
    (current.x < goal.x && proposed.x > current.x)
        || (current.y < goal.y && proposed.y > current.y)
}

fn span_deficit(current: IVec2, goal: IVec2) -> i32 {
    (goal.x - current.x).max(0) + (goal.y - current.y).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_zero_contains_only_origin() {
        assert_eq!(ring_cells(0), vec![IVec2::ZERO]);
    }

    #[test]
    fn every_ring_contains_exactly_its_chebyshev_perimeter() {
        for radius in 1..=8 {
            let cells = ring_cells(radius);
            assert_eq!(cells.len(), (radius * 8) as usize);
            assert!(
                cells
                    .iter()
                    .all(|cell| cell.x.abs().max(cell.y.abs()) == radius)
            );
        }
    }

    #[test]
    fn authored_surface_radii_convert_to_full_map_spans() {
        let axis = DimensionBiomeSizeAxis {
            min: 80.0,
            max: 180.0,
        };
        assert_eq!(axis_cell_limits(axis, 10.0), (16, 36));
    }

    #[test]
    fn target_span_never_leaves_authored_cell_limits() {
        for unit in [0.0, 0.1, 0.5, 0.999_999, 1.0] {
            let target = choose_target_span(8, 19, unit);
            assert!((8..=19).contains(&target));
        }
    }
}
