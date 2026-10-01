use std::{
    collections::VecDeque,
    sync::{Arc, LockResult, Mutex, MutexGuard, RwLock, RwLockReadGuard},
};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{voxel::chunk::CHUNK_SIZE, world::deterministic::mix_hash_u64};

use super::{
    BiomeField, BiomeFieldEntry, SurfaceSiteCacheEntry,
    constants::SITE_SEARCH_RADIUS,
    distribution::distribution_strength,
    selection::{
        climate_weight, fitting::fit_surface_site_weights, region_claim_hash,
        surface_sites_share_border,
    },
    spatial::{cell_hash, hash_unit, surface_site_position},
};

const MAP_FIT_EPSILON: f32 = 0.001;

/// Canonical surface-biome site map plus disposable fitted-boundary weights.
///
/// Site identities are resolved before terrain sampling. Once a site has been
/// consumed by a sampling window it becomes stable and is never evicted or
/// reassigned. The surrounding unresolved halo is kept as provisional state
/// and may be solved again when a later window needs to propagate constraints
/// farther. Fitted power-diagram weights remain disposable acceleration.
#[derive(Clone, Default)]
pub(crate) struct SurfaceSiteCache {
    entries: Arc<RwLock<HashMap<IVec2, SurfaceSiteCacheEntry>>>,
    stable: Arc<RwLock<HashSet<IVec2>>>,
    fitted_weights: Arc<RwLock<HashMap<IVec2, Arc<[f32]>>>>,
    resolution: Arc<Mutex<()>>,
}

impl SurfaceSiteCache {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn read(
        &self,
    ) -> LockResult<RwLockReadGuard<'_, HashMap<IVec2, SurfaceSiteCacheEntry>>> {
        self.entries.read()
    }

    fn resolution_guard(&self) -> MutexGuard<'_, ()> {
        self.resolution
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn snapshot(&self) -> (HashMap<IVec2, SurfaceSiteCacheEntry>, HashSet<IVec2>) {
        let entries = self
            .entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let stable = self
            .stable
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        (entries, stable)
    }

    fn cells_are_stable(&self, cells: &[IVec2]) -> bool {
        let stable = self
            .stable
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cells.iter().all(|cell| stable.contains(cell))
    }

    fn store_resolution(
        &self,
        resolved: &[(IVec2, SurfaceSiteCacheEntry)],
        newly_stable: &[IVec2],
    ) {
        let stable_snapshot = self
            .stable
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let mut changed = false;
        {
            let mut entries = self
                .entries
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for &(cell, entry) in resolved {
                if stable_snapshot.contains(&cell) {
                    let existing = entries
                        .get(&cell)
                        .expect("stable surface-biome site must have a map entry");
                    debug_assert_eq!(existing.biome_index, entry.biome_index);
                    continue;
                }
                changed |= entries.get(&cell).is_none_or(|existing| {
                    existing.biome_index != entry.biome_index || existing.position != entry.position
                });
                entries.insert(cell, entry);
            }
        }
        self.stable
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .extend(newly_stable.iter().copied());

        if changed {
            self.fitted_weights
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clear();
        }
    }

    pub(super) fn fitted_weights(&self, center: IVec2) -> Option<Arc<[f32]>> {
        self.fitted_weights
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&center)
            .cloned()
    }

    pub(super) fn cache_fitted_weights(&self, center: IVec2, weights: Vec<f32>) -> Arc<[f32]> {
        let weights = Arc::<[f32]>::from(weights);
        let mut cache = self
            .fitted_weights
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache
            .entry(center)
            .or_insert_with(|| Arc::clone(&weights))
            .clone()
    }

    pub(super) fn clear(&self) {
        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.stable
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.fitted_weights
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    pub(super) fn retain_around(
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
        let retain = |cell: &IVec2| {
            (cell.x - center_cell.x).abs() <= radius_x
                && (cell.y - center_cell.y).abs() <= radius_z
        };
        let stable = self
            .stable
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();

        // Stable map identities are world state. Only provisional halo entries
        // and fitted-weight acceleration may be evicted by streaming.
        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|cell, _| stable.contains(cell) || retain(cell));
        self.fitted_weights
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|cell, _| retain(cell));
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    #[cfg(test)]
    fn fitted_len(&self) -> usize {
        self.fitted_weights
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

#[derive(Clone)]
struct SurfaceMapNode {
    cell: IVec2,
    site: Vec2,
    candidates: Vec<usize>,
    fixed: bool,
}

struct SurfaceMapSolver<'a> {
    field: &'a BiomeField,
    nodes: Vec<SurfaceMapNode>,
    by_cell: HashMap<IVec2, usize>,
    context_entries: &'a HashMap<IVec2, SurfaceSiteCacheEntry>,
    protected_stable: Vec<IVec2>,
    target_cells: &'a [IVec2],
    nogoods: &'a [Vec<(IVec2, usize)>],
    assignments: Vec<Option<usize>>,
    variable_order: Vec<usize>,
}

impl BiomeField {
    /// Resolves the complete canonical 5x5 site window needed by one surface
    /// sample before any terrain/chunk code consumes those identities.
    pub(super) fn ensure_surface_map_window(&self, center: IVec2) {
        if self.single_surface_biome.is_some() {
            return;
        }

        let target_cells = surface_window_cells(center, SITE_SEARCH_RADIUS);
        let _resolution = self.surface_site_cache.resolution_guard();
        if self.surface_site_cache.cells_are_stable(&target_cells) {
            return;
        }

        let (context_entries, stable_cells) = self.surface_site_cache.snapshot();
        let work_radius = SITE_SEARCH_RADIUS * 2;
        let mut nodes = Vec::new();
        for z in -work_radius..=work_radius {
            for x in -work_radius..=work_radius {
                let cell = center + IVec2::new(x, z);
                let site = surface_site_position(cell, self.surface_site_spacing, self.seed);
                let fixed = stable_cells.contains(&cell);
                let candidates = if fixed {
                    vec![context_entries
                        .get(&cell)
                        .expect("stable surface-biome site must have a map entry")
                        .biome_index]
                } else {
                    self.surface_map_candidates(cell, site)
                };
                assert!(
                    !candidates.is_empty(),
                    "surface biome map site {cell:?} has no biome compatible with climate/distribution constraints"
                );
                nodes.push(SurfaceMapNode {
                    cell,
                    site,
                    candidates,
                    fixed,
                });
            }
        }

        let protected_stable = stable_cells
            .iter()
            .copied()
            .filter(|cell| {
                (cell.x - center.x).abs() <= work_radius + SITE_SEARCH_RADIUS
                    && (cell.y - center.y).abs() <= work_radius + SITE_SEARCH_RADIUS
            })
            .collect::<Vec<_>>();
        let mut nogoods = Vec::<Vec<(IVec2, usize)>>::new();
        let mut last_fit_error = None;

        loop {
            let solver = SurfaceMapSolver::new(
                self,
                nodes.clone(),
                &context_entries,
                protected_stable.clone(),
                &target_cells,
                &nogoods,
            );
            let Some(assignments) = solver.solve() else {
                let suffix = last_fit_error
                    .as_deref()
                    .map(|reason| format!("; last boundary fit error: {reason}"))
                    .unwrap_or_default();
                panic!(
                    "surface biome map around site {center:?} has no assignment satisfying authored adjacency and size constraints{suffix}"
                );
            };

            let by_cell = nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node.cell, index))
                .collect::<HashMap<_, _>>();
            let samples = target_cells
                .iter()
                .map(|cell| {
                    let node_index = *by_cell
                        .get(cell)
                        .expect("surface biome map target must be inside work region");
                    (
                        *cell,
                        nodes[node_index].site,
                        0.0,
                        Some(assignments[node_index]),
                    )
                })
                .collect::<Vec<_>>();

            match fit_surface_site_weights(
                &samples,
                &self.surface_biomes,
                self.surface_site_spacing,
                self.seed,
            ) {
                Ok(weights) => {
                    let resolved = nodes
                        .iter()
                        .enumerate()
                        .map(|(index, node)| {
                            (
                                node.cell,
                                SurfaceSiteCacheEntry {
                                    position: node.site,
                                    biome_index: assignments[index],
                                },
                            )
                        })
                        .collect::<Vec<_>>();
                    self.surface_site_cache
                        .store_resolution(&resolved, &target_cells);
                    self.surface_site_cache
                        .cache_fitted_weights(center, weights);
                    return;
                }
                Err(reason) => {
                    last_fit_error = Some(reason);
                    nogoods.push(
                        target_cells
                            .iter()
                            .map(|cell| {
                                let node_index = *by_cell
                                    .get(cell)
                                    .expect("surface biome map target must be inside work region");
                                (*cell, assignments[node_index])
                            })
                            .collect(),
                    );
                }
            }
        }
    }

    fn surface_map_candidates(&self, cell: IVec2, site: Vec2) -> Vec<usize> {
        let climate = self.climate.sample(site);

        // Keep the authored ocean continentalness core authoritative, matching
        // the legacy selector. The map solver still resolves shoreline cells.
        if let Some(ocean_index) = self.ocean_surface_index
            && self.surface_biome_is_enabled(ocean_index)
        {
            let ocean = &self.surface_biomes[ocean_index];
            if ocean.weight > f32::EPSILON
                && ocean
                    .distributions
                    .iter()
                    .copied()
                    .any(|distribution| distribution.is_regional())
                && climate_weight(climate, ocean.climate) >= 1.0 - f32::EPSILON
            {
                return vec![ocean_index];
            }
        }

        let source_hash = cell_hash(cell, self.seed);
        let mut candidates = self
            .surface_biomes
            .iter()
            .enumerate()
            .filter(|(index, biome)| {
                biome.weight > 0.0 && self.surface_biome_is_enabled(*index)
            })
            .filter_map(|(index, biome)| {
                let distribution = biome
                    .distributions
                    .iter()
                    .copied()
                    .map(|distribution| {
                        distribution_strength(distribution, site, self.seed, biome.id.as_str())
                    })
                    .fold(0.0_f32, f32::max);
                if distribution <= 0.0 {
                    return None;
                }
                let climate_weight = climate_weight(climate, biome.climate);
                if climate_weight <= 0.0 {
                    return None;
                }
                Some((index, biome.weight * climate_weight * distribution))
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|(left_index, left_weight), (right_index, right_weight)| {
            let left_hash = surface_map_candidate_hash(
                source_hash,
                self.surface_biomes[*left_index].id.as_str(),
            );
            let right_hash = surface_map_candidate_hash(
                source_hash,
                self.surface_biomes[*right_index].id.as_str(),
            );
            let left_score = -hash_unit(left_hash).ln() / left_weight.max(f32::MIN_POSITIVE);
            let right_score = -hash_unit(right_hash).ln() / right_weight.max(f32::MIN_POSITIVE);
            left_score
                .total_cmp(&right_score)
                .then_with(|| left_index.cmp(right_index))
        });
        candidates.into_iter().map(|(index, _)| index).collect()
    }
}

impl<'a> SurfaceMapSolver<'a> {
    fn new(
        field: &'a BiomeField,
        nodes: Vec<SurfaceMapNode>,
        context_entries: &'a HashMap<IVec2, SurfaceSiteCacheEntry>,
        protected_stable: Vec<IVec2>,
        target_cells: &'a [IVec2],
        nogoods: &'a [Vec<(IVec2, usize)>],
    ) -> Self {
        let by_cell = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.cell, index))
            .collect::<HashMap<_, _>>();
        let assignments = nodes
            .iter()
            .map(|node| node.fixed.then_some(node.candidates[0]))
            .collect::<Vec<_>>();
        let mut variable_order = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| !node.fixed)
            .map(|(index, node)| {
                let first = node.candidates[0];
                (
                    index,
                    node.candidates.len(),
                    region_claim_hash(node.cell, first, field.seed),
                    node.cell.y,
                    node.cell.x,
                )
            })
            .collect::<Vec<_>>();
        variable_order.sort_by_key(|(_, domain, claim, y, x)| (*domain, *claim, *y, *x));
        let variable_order = variable_order
            .into_iter()
            .map(|(index, _, _, _, _)| index)
            .collect();

        Self {
            field,
            nodes,
            by_cell,
            context_entries,
            protected_stable,
            target_cells,
            nogoods,
            assignments,
            variable_order,
        }
    }

    fn solve(mut self) -> Option<Vec<usize>> {
        self.search(0).then(|| {
            self.assignments
                .into_iter()
                .map(|assignment| assignment.expect("solved surface map node must be assigned"))
                .collect()
        })
    }

    fn search(&mut self, order_index: usize) -> bool {
        if order_index == self.variable_order.len() {
            return self.final_constraints_allow();
        }

        let node_index = self.variable_order[order_index];
        let candidates = self.nodes[node_index].candidates.clone();
        for candidate in candidates {
            self.assignments[node_index] = Some(candidate);
            if self.local_constraints_allow(node_index, candidate)
                && self.search(order_index + 1)
            {
                return true;
            }
            self.assignments[node_index] = None;
        }
        false
    }

    fn local_constraints_allow(&self, node_index: usize, candidate_index: usize) -> bool {
        let node = &self.nodes[node_index];
        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let neighbor_cell = node.cell + offset;
                let Some(neighbor_index) = self.resolved_index(neighbor_cell) else {
                    continue;
                };
                let neighbor_site = self.site_for_cell(neighbor_cell);
                if !surface_sites_share_border(
                    node.cell,
                    node.site,
                    neighbor_cell,
                    neighbor_site,
                    self.field.surface_site_spacing,
                    self.field.seed,
                ) {
                    continue;
                }
                if !surface_map_pair_allows(
                    &self.field.surface_biomes[candidate_index],
                    &self.field.surface_biomes[neighbor_index],
                    node.site,
                    neighbor_site,
                ) {
                    return false;
                }
            }
        }

        self.component_size_allows(node.cell, candidate_index)
    }

    fn component_size_allows(&self, start: IVec2, candidate_index: usize) -> bool {
        if self.field.ocean_surface_index == Some(candidate_index) {
            return true;
        }
        let biome = &self.field.surface_biomes[candidate_index];
        let span_limit = Vec2::new(
            ((biome.size.x.max - biome.size.x.min) * 2.0).max(0.0),
            ((biome.size.z.max - biome.size.z.min) * 2.0).max(0.0),
        );
        let mut queue = VecDeque::new();
        queue.push_back(start);
        let mut seen = HashSet::new();
        seen.insert(start);
        let mut min = self.site_for_cell(start);
        let mut max = min;

        while let Some(cell) = queue.pop_front() {
            let site = self.site_for_cell(cell);
            min = min.min(site);
            max = max.max(site);
            let span = max - min;
            if span.x > span_limit.x + MAP_FIT_EPSILON
                || span.y > span_limit.y + MAP_FIT_EPSILON
            {
                return false;
            }

            for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                    let offset = IVec2::new(x, z);
                    if offset == IVec2::ZERO {
                        continue;
                    }
                    let neighbor_cell = cell + offset;
                    if seen.contains(&neighbor_cell)
                        || self.resolved_index(neighbor_cell) != Some(candidate_index)
                    {
                        continue;
                    }
                    let neighbor_site = self.site_for_cell(neighbor_cell);
                    if !surface_sites_share_border(
                        cell,
                        site,
                        neighbor_cell,
                        neighbor_site,
                        self.field.surface_site_spacing,
                        self.field.seed,
                    ) {
                        continue;
                    }
                    seen.insert(neighbor_cell);
                    queue.push_back(neighbor_cell);
                }
            }
        }

        true
    }

    fn final_constraints_allow(&self) -> bool {
        if self.nogoods.iter().any(|nogood| {
            nogood
                .iter()
                .all(|(cell, biome)| self.resolved_index(*cell) == Some(*biome))
        }) {
            return false;
        }

        for &cell in self.target_cells.iter().chain(&self.protected_stable) {
            let Some(index) = self.resolved_index(cell) else {
                continue;
            };
            if !self.require_near_allows(cell, index) {
                return false;
            }
        }
        true
    }

    fn require_near_allows(&self, cell: IVec2, candidate_index: usize) -> bool {
        let candidate = &self.field.surface_biomes[candidate_index];
        if candidate.require_near.is_empty() {
            return true;
        }
        let site = self.site_for_cell(cell);

        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let neighbor_cell = cell + offset;
                let Some(neighbor_index) = self.resolved_index(neighbor_cell) else {
                    continue;
                };
                let neighbor_site = self.site_for_cell(neighbor_cell);
                if !surface_sites_share_border(
                    cell,
                    site,
                    neighbor_cell,
                    neighbor_site,
                    self.field.surface_site_spacing,
                    self.field.seed,
                ) {
                    continue;
                }
                if candidate
                    .require_near
                    .iter()
                    .any(|id| id == &self.field.surface_biomes[neighbor_index].id)
                {
                    return true;
                }
            }
        }
        false
    }

    fn resolved_index(&self, cell: IVec2) -> Option<usize> {
        if let Some(&node_index) = self.by_cell.get(&cell) {
            return self.assignments[node_index];
        }
        self.context_entries
            .get(&cell)
            .map(|entry| entry.biome_index)
    }

    fn site_for_cell(&self, cell: IVec2) -> Vec2 {
        if let Some(&node_index) = self.by_cell.get(&cell) {
            return self.nodes[node_index].site;
        }
        self.context_entries.get(&cell).map_or_else(
            || surface_site_position(cell, self.field.surface_site_spacing, self.field.seed),
            |entry| entry.position,
        )
    }
}

fn surface_window_cells(center: IVec2, radius: i32) -> Vec<IVec2> {
    let diameter = (radius * 2 + 1) as usize;
    let mut cells = Vec::with_capacity(diameter * diameter);
    for z in -radius..=radius {
        for x in -radius..=radius {
            cells.push(center + IVec2::new(x, z));
        }
    }
    cells
}

fn surface_map_candidate_hash(source_hash: u64, biome_id: &str) -> u64 {
    let mut hash = source_hash;
    for byte in biome_id.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    mix_hash_u64(hash)
}

fn surface_map_pair_allows(
    left: &BiomeFieldEntry,
    right: &BiomeFieldEntry,
    left_site: Vec2,
    right_site: Vec2,
) -> bool {
    if authored_pair_conflicts(left, right) {
        return false;
    }
    left.id == right.id || surface_boundary_has_fit(left, right, left_site, right_site)
}

fn authored_pair_conflicts(left: &BiomeFieldEntry, right: &BiomeFieldEntry) -> bool {
    if left.avoid_near.iter().any(|id| id == &right.id)
        || right.avoid_near.iter().any(|id| id == &left.id)
    {
        return true;
    }
    left.id != right.id
        && left.exclusive_neighbor_group.as_ref().is_some_and(|group| {
            right
                .exclusive_neighbor_group
                .as_ref()
                .is_some_and(|right_group| right_group == group)
        })
}

fn surface_boundary_has_fit(
    left: &BiomeFieldEntry,
    right: &BiomeFieldEntry,
    left_site: Vec2,
    right_site: Vec2,
) -> bool {
    let delta = right_site - left_site;
    let distance = delta.length();
    if distance <= MAP_FIT_EPSILON {
        return false;
    }
    let direction = delta / distance;
    let left_min = directional_radius(left.size.x.min, left.size.z.min, direction);
    let left_max = directional_radius(left.size.x.max, left.size.z.max, direction);
    let right_min = directional_radius(right.size.x.min, right.size.z.min, direction);
    let right_max = directional_radius(right.size.x.max, right.size.z.max, direction);
    let min_from_left = left_min.max(distance - right_max).max(0.0);
    let max_from_left = left_max.min(distance - right_min).min(distance);
    min_from_left <= max_from_left + MAP_FIT_EPSILON
}

fn directional_radius(radius_x: f32, radius_z: f32, direction: Vec2) -> f32 {
    let radius_x = radius_x.max(MAP_FIT_EPSILON);
    let radius_z = radius_z.max(MAP_FIT_EPSILON);
    1.0 / ((direction.x / radius_x).powi(2) + (direction.y / radius_z).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: usize) -> SurfaceSiteCacheEntry {
        SurfaceSiteCacheEntry {
            position: Vec2::ZERO,
            biome_index: index,
        }
    }

    #[test]
    fn clones_share_entries_and_invalidation() {
        let cache = SurfaceSiteCache::new();
        cache
            .entries
            .write()
            .expect("surface-site cache write lock should not be poisoned")
            .insert(IVec2::new(2, -3), entry(4));
        cache.cache_fitted_weights(IVec2::new(2, -3), vec![0.0, 1.0]);

        let clone = cache.clone();
        assert_eq!(clone.len(), 1);
        assert_eq!(clone.fitted_len(), 1);

        clone.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.fitted_len(), 0);
    }

    #[test]
    fn retention_drops_provisional_sites_and_fits_outside_streaming_bound() {
        let cache = SurfaceSiteCache::new();
        {
            let mut entries = cache
                .entries
                .write()
                .expect("surface-site cache write lock should not be poisoned");
            entries.insert(IVec2::ZERO, entry(0));
            entries.insert(IVec2::new(512, 0), entry(1));
        }
        cache.cache_fitted_weights(IVec2::ZERO, vec![0.0]);
        cache.cache_fitted_weights(IVec2::new(512, 0), vec![1.0]);

        cache.retain_around(IVec2::ZERO, 4, Vec2::splat(128.0));

        let entries = cache
            .read()
            .expect("surface-site cache read lock should not be poisoned");
        assert!(entries.contains_key(&IVec2::ZERO));
        assert!(!entries.contains_key(&IVec2::new(512, 0)));
        drop(entries);

        assert!(cache.fitted_weights(IVec2::ZERO).is_some());
        assert!(cache.fitted_weights(IVec2::new(512, 0)).is_none());
    }

    #[test]
    fn retention_keeps_stable_map_sites_outside_streaming_bound() {
        let cache = SurfaceSiteCache::new();
        let far = IVec2::new(512, 0);
        cache.store_resolution(&[(far, entry(1))], &[far]);

        cache.retain_around(IVec2::ZERO, 4, Vec2::splat(128.0));

        assert!(cache
            .read()
            .expect("surface-site cache read lock should not be poisoned")
            .contains_key(&far));
        assert!(cache.cells_are_stable(&[far]));
    }
}
