use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    sync::Mutex,
};

use bevy::prelude::*;

use super::{
    authored_pair_conflicts, cell_hash, climate_weight, fitting, region_claim_hash,
    surface_site_position, surface_sites_share_border,
};
use crate::world::biome_field::{BiomeField, SurfaceSiteCacheEntry};

const FIT_EPSILON: f32 = 0.001;
type SolverSignature = Vec<(i32, i32, Vec<usize>)>;

// Chunk generation may run several workers over overlapping columns. Only an
// unresolved surface graph needs serialization: once one worker publishes the
// canonical site identities, later workers return from the shared cache.
static SURFACE_SOLVER_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone)]
struct CellState {
    site: Vec2,
    domain: Vec<usize>,
}

impl CellState {
    fn biome_index(&self) -> usize {
        self.domain[0]
    }
}

#[derive(Clone, Default)]
struct SolverState {
    cells: HashMap<IVec2, CellState>,
}

impl SolverState {
    fn signature(&self) -> SolverSignature {
        let mut cells = self
            .cells
            .iter()
            .map(|(cell, state)| (cell.x, cell.y, state.domain.clone()))
            .collect::<Vec<_>>();
        cells.sort_by_key(|(x, y, _)| (*y, *x));
        cells
    }
}

#[derive(Clone, Copy)]
struct SiteBounds {
    min: Vec2,
    max: Vec2,
}

impl SiteBounds {
    fn from_site(site: Vec2) -> Self {
        Self {
            min: site,
            max: site,
        }
    }

    fn include(&mut self, site: Vec2) {
        self.min = self.min.min(site);
        self.max = self.max.max(site);
    }

    fn exceeds(self, limit: Vec2) -> bool {
        let span = self.max - self.min;
        span.x > limit.x + FIT_EPSILON || span.y > limit.y + FIT_EPSILON
    }
}

enum Violation {
    Pair { left: IVec2, right: IVec2 },
    Oversize { cells: Vec<IVec2> },
    RequireNear { cell: IVec2 },
    BoundaryCycle { cells: Vec<IVec2> },
}

struct RecursiveSurfaceSolver<'a> {
    field: &'a BiomeField,
    candidate_cache: RefCell<HashMap<IVec2, Vec<usize>>>,
    neighbor_cache: RefCell<HashMap<IVec2, Vec<(IVec2, Vec2)>>>,
}

/// Resolves a group of surface sites as one recursive constraint problem.
///
/// Pairwise arc consistency prunes impossible candidates before branching. When
/// propagation still leaves an authored size/adjacency conflict, search branches
/// on one minimum-remaining-values cell and fixes it to one candidate. This is
/// complete backtracking, but avoids the combinatorial duplicate search caused
/// by branching once for every cell in a conflicting component.
pub(in crate::world::biome_field) fn resolve_surface_sites(
    field: &BiomeField,
    requested_cells: &[IVec2],
) -> Result<Vec<(IVec2, Vec2, usize)>, String> {
    let _solver_guard = SURFACE_SOLVER_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(resolved) = resolved_requested_sites_from_cache(field, requested_cells) {
        return Ok(resolved);
    }

    let solver = RecursiveSurfaceSolver {
        field,
        candidate_cache: RefCell::default(),
        neighbor_cache: RefCell::default(),
    };
    let mut state = SolverState::default();
    for &cell in requested_cells {
        solver.ensure_cell(&mut state, cell)?;
    }

    let mut failed_states = HashSet::<SolverSignature>::new();
    let solved = solver.solve(state, &mut failed_states).ok_or_else(|| {
        let first = requested_cells.first().copied().unwrap_or(IVec2::ZERO);
        format!(
            "surface biome constraints have no globally consistent assignment near site {first:?}"
        )
    })?;

    let mut resolved = solved
        .cells
        .into_iter()
        .map(|(cell, state)| (cell, state.site, state.biome_index()))
        .collect::<Vec<_>>();
    resolved.sort_by_key(|(cell, _, _)| (cell.y, cell.x));

    {
        let mut cache = field
            .surface_site_cache
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for &(cell, site, selected) in &resolved {
            cache
                .entry(cell)
                .and_modify(|existing| {
                    debug_assert_eq!(
                        existing.biome_index, selected,
                        "recursive biome fitting must be independent of query order"
                    );
                })
                .or_insert(SurfaceSiteCacheEntry {
                    position: site,
                    biome_index: selected,
                });
        }
    }

    Ok(resolved)
}

fn resolved_requested_sites_from_cache(
    field: &BiomeField,
    requested_cells: &[IVec2],
) -> Option<Vec<(IVec2, Vec2, usize)>> {
    let cache = field
        .surface_site_cache
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut resolved = Vec::with_capacity(requested_cells.len());
    for &cell in requested_cells {
        let entry = cache.get(&cell).copied()?;
        resolved.push((cell, entry.position, entry.biome_index));
    }
    resolved.sort_by_key(|(cell, _, _)| (cell.y, cell.x));
    Some(resolved)
}

impl RecursiveSurfaceSolver<'_> {
    fn solve(
        &self,
        mut state: SolverState,
        failed_states: &mut HashSet<SolverSignature>,
    ) -> Option<SolverState> {
        if !self.propagate_pair_domains(&mut state) {
            return None;
        }
        if self.expand_first_pair_conflict(&mut state).is_err() {
            return None;
        }
        if !self.propagate_pair_domains(&mut state) {
            return None;
        }

        let signature = state.signature();
        if failed_states.contains(&signature) {
            return None;
        }

        let oversize = match self.first_oversize_violation(&mut state) {
            Ok(oversize) => oversize,
            Err(_) => {
                failed_states.insert(signature);
                return None;
            }
        };
        let require_near = if oversize.is_none() && self.first_pair_violation(&state).is_none() {
            match self.first_require_near_violation(&mut state) {
                Ok(violation) => violation,
                Err(_) => {
                    failed_states.insert(signature);
                    return None;
                }
            }
        } else {
            None
        };

        if !self.propagate_pair_domains(&mut state) {
            failed_states.insert(signature);
            return None;
        }

        let violation = if let Some((left, right)) = self.first_pair_violation(&state) {
            Some(Violation::Pair { left, right })
        } else if let Some(cells) = oversize {
            Some(Violation::Oversize { cells })
        } else if let Some(cell) = require_near {
            Some(Violation::RequireNear { cell })
        } else {
            self.boundary_cycle_violation(&state)
                .map(|cells| Violation::BoundaryCycle { cells })
        };

        let Some(violation) = violation else {
            return Some(state);
        };

        for branch in self.repair_branches(&state, violation) {
            if let Some(solved) = self.solve(branch, failed_states) {
                return Some(solved);
            }
        }
        failed_states.insert(signature);
        None
    }

    fn ensure_cell(&self, state: &mut SolverState, cell: IVec2) -> Result<(), String> {
        if state.cells.contains_key(&cell) {
            return Ok(());
        }

        if let Some(cached) = self
            .field
            .surface_site_cache
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&cell)
            .copied()
        {
            state.cells.insert(
                cell,
                CellState {
                    site: cached.position,
                    domain: vec![cached.biome_index],
                },
            );
            return Ok(());
        }

        let site = surface_site_position(cell, self.field.surface_site_spacing, self.field.seed);
        let domain = self.candidate_order(cell, site)?;
        state.cells.insert(cell, CellState { site, domain });
        Ok(())
    }

    fn candidate_order(&self, cell: IVec2, site: Vec2) -> Result<Vec<usize>, String> {
        if let Some(cached) = self.candidate_cache.borrow().get(&cell).cloned() {
            return Ok(cached);
        }

        let climate = self.field.climate.sample(site);
        let candidates = if let Some(ocean_index) = self.field.ocean_surface_index
            && self.field.surface_biome_is_enabled(ocean_index)
        {
            let ocean = &self.field.surface_biomes[ocean_index];
            if ocean.weight > f32::EPSILON
                && ocean
                    .distributions
                    .iter()
                    .copied()
                    .any(|distribution| distribution.is_regional())
                && climate_weight(climate, ocean.climate) >= 1.0 - f32::EPSILON
            {
                vec![ocean_index]
            } else {
                self.weighted_candidates(cell, site, climate)
            }
        } else {
            self.weighted_candidates(cell, site, climate)
        };

        if candidates.is_empty() {
            return Err(format!(
                "surface biome site {cell:?} has no biome compatible with climate/distribution constraints"
            ));
        }
        self.candidate_cache
            .borrow_mut()
            .insert(cell, candidates.clone());
        Ok(candidates)
    }

    fn weighted_candidates(
        &self,
        cell: IVec2,
        site: Vec2,
        climate: crate::world::macro_climate::MacroClimateSample,
    ) -> Vec<usize> {
        self.field
            .surface_weighted_candidates(cell, site, climate, cell_hash(cell, self.field.seed))
            .into_iter()
            .map(|candidate| candidate.index)
            .collect()
    }

    fn raw_identity(&self, cell: IVec2, site: Vec2) -> Result<usize, String> {
        self.candidate_order(cell, site).map(|domain| domain[0])
    }

    fn identity_at(&self, state: &SolverState, cell: IVec2, site: Vec2) -> Result<usize, String> {
        state
            .cells
            .get(&cell)
            .map(CellState::biome_index)
            .map_or_else(|| self.raw_identity(cell, site), Ok)
    }

    fn neighbors(&self, cell: IVec2, site: Vec2) -> Vec<(IVec2, Vec2)> {
        if let Some(cached) = self.neighbor_cache.borrow().get(&cell).cloned() {
            return cached;
        }

        let mut neighbors = Vec::new();
        for z in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
            for x in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let neighbor_cell = cell + offset;
                let neighbor_site = surface_site_position(
                    neighbor_cell,
                    self.field.surface_site_spacing,
                    self.field.seed,
                );
                if surface_sites_share_border(
                    cell,
                    site,
                    neighbor_cell,
                    neighbor_site,
                    self.field.surface_site_spacing,
                    self.field.seed,
                ) {
                    neighbors.push((neighbor_cell, neighbor_site));
                }
            }
        }
        neighbors.sort_by_key(|(neighbor, _)| (neighbor.y, neighbor.x));
        self.neighbor_cache
            .borrow_mut()
            .insert(cell, neighbors.clone());
        neighbors
    }

    fn pair_allows(
        &self,
        left_index: usize,
        left_site: Vec2,
        right_index: usize,
        right_site: Vec2,
    ) -> bool {
        if left_index == right_index {
            return true;
        }
        let left = &self.field.surface_biomes[left_index];
        let right = &self.field.surface_biomes[right_index];
        !authored_pair_conflicts(left, right)
            && fitting::boundary_fit_interval(left, right, left_site, right_site).is_some()
    }

    fn propagate_pair_domains(&self, state: &mut SolverState) -> bool {
        loop {
            let mut changed = false;
            let mut cells = state.cells.keys().copied().collect::<Vec<_>>();
            cells.sort_by_key(|cell| (cell.y, cell.x));

            for cell in cells {
                let left_site = state.cells[&cell].site;
                for (neighbor_cell, neighbor_site) in self.neighbors(cell, left_site) {
                    if (neighbor_cell.y, neighbor_cell.x) <= (cell.y, cell.x) {
                        continue;
                    }
                    let Some(right) = state.cells.get(&neighbor_cell) else {
                        continue;
                    };

                    let left_domain = state.cells[&cell].domain.clone();
                    let right_domain = right.domain.clone();
                    let filtered_left = left_domain
                        .iter()
                        .copied()
                        .filter(|left_index| {
                            right_domain.iter().copied().any(|right_index| {
                                self.pair_allows(
                                    *left_index,
                                    left_site,
                                    right_index,
                                    neighbor_site,
                                )
                            })
                        })
                        .collect::<Vec<_>>();
                    if filtered_left.is_empty() {
                        return false;
                    }
                    let filtered_right = right_domain
                        .iter()
                        .copied()
                        .filter(|right_index| {
                            filtered_left.iter().copied().any(|left_index| {
                                self.pair_allows(
                                    left_index,
                                    left_site,
                                    *right_index,
                                    neighbor_site,
                                )
                            })
                        })
                        .collect::<Vec<_>>();
                    if filtered_right.is_empty() {
                        return false;
                    }

                    if filtered_left != left_domain {
                        state
                            .cells
                            .get_mut(&cell)
                            .expect("solver cell must exist")
                            .domain = filtered_left;
                        changed = true;
                    }
                    if filtered_right != right_domain {
                        state
                            .cells
                            .get_mut(&neighbor_cell)
                            .expect("solver neighbor must exist")
                            .domain = filtered_right;
                        changed = true;
                    }
                }
            }

            if !changed {
                return true;
            }
        }
    }

    fn expand_first_pair_conflict(&self, state: &mut SolverState) -> Result<(), String> {
        let mut cells = state.cells.keys().copied().collect::<Vec<_>>();
        cells.sort_by_key(|cell| (cell.y, cell.x));

        for cell in cells {
            let left = state.cells.get(&cell).expect("solver cell must exist");
            let left_site = left.site;
            let left_index = left.biome_index();
            for (neighbor_cell, neighbor_site) in self.neighbors(cell, left_site) {
                if state.cells.contains_key(&neighbor_cell) {
                    continue;
                }
                let right_index = self.raw_identity(neighbor_cell, neighbor_site)?;
                if self.pair_allows(left_index, left_site, right_index, neighbor_site) {
                    continue;
                }

                self.ensure_cell(state, neighbor_cell)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn first_pair_violation(&self, state: &SolverState) -> Option<(IVec2, IVec2)> {
        let mut cells = state.cells.keys().copied().collect::<Vec<_>>();
        cells.sort_by_key(|cell| (cell.y, cell.x));

        for cell in cells {
            let left = state.cells.get(&cell).expect("solver cell must exist");
            for (neighbor_cell, neighbor_site) in self.neighbors(cell, left.site) {
                if (neighbor_cell.y, neighbor_cell.x) <= (cell.y, cell.x) {
                    continue;
                }
                let Some(right) = state.cells.get(&neighbor_cell) else {
                    continue;
                };
                if !self.pair_allows(
                    left.biome_index(),
                    left.site,
                    right.biome_index(),
                    neighbor_site,
                ) {
                    return Some((cell, neighbor_cell));
                }
            }
        }
        None
    }

    fn first_oversize_violation(
        &self,
        state: &mut SolverState,
    ) -> Result<Option<Vec<IVec2>>, String> {
        let mut starts = state.cells.keys().copied().collect::<Vec<_>>();
        starts.sort_by_key(|cell| (cell.y, cell.x));
        let mut checked = HashSet::new();

        for start in starts {
            if checked.contains(&start) {
                continue;
            }
            let start_state = state.cells.get(&start).expect("solver cell must exist");
            let biome_index = start_state.biome_index();
            if self.field.ocean_surface_index == Some(biome_index) {
                checked.insert(start);
                continue;
            }

            let biome = &self.field.surface_biomes[biome_index];
            let center_span_limit = Vec2::new(
                ((biome.size.x.max - biome.size.x.min) * 2.0).max(0.0),
                ((biome.size.z.max - biome.size.z.min) * 2.0).max(0.0),
            );
            let mut bounds = SiteBounds::from_site(start_state.site);
            let mut queue = VecDeque::from([(start, start_state.site)]);
            let mut seen = HashSet::from([start]);
            let mut component = Vec::new();

            while let Some((cell, site)) = queue.pop_front() {
                component.push(cell);
                bounds.include(site);

                if bounds.exceeds(center_span_limit) {
                    for &component_cell in &component {
                        self.ensure_cell(state, component_cell)?;
                    }
                    component.sort_by_key(|component_cell| (component_cell.y, component_cell.x));
                    return Ok(Some(component));
                }

                for (neighbor_cell, neighbor_site) in self.neighbors(cell, site) {
                    if !seen.insert(neighbor_cell) {
                        continue;
                    }
                    let neighbor_index = self.identity_at(state, neighbor_cell, neighbor_site)?;
                    if neighbor_index == biome_index {
                        queue.push_back((neighbor_cell, neighbor_site));
                    }
                }
            }

            for cell in component {
                if state.cells.contains_key(&cell) {
                    checked.insert(cell);
                }
            }
        }
        Ok(None)
    }

    fn first_require_near_violation(
        &self,
        state: &mut SolverState,
    ) -> Result<Option<IVec2>, String> {
        let mut cells = state.cells.keys().copied().collect::<Vec<_>>();
        cells.sort_by_key(|cell| (cell.y, cell.x));

        for cell in cells {
            let current = state.cells.get(&cell).expect("solver cell must exist");
            let biome = &self.field.surface_biomes[current.biome_index()];
            if biome.require_near.is_empty() {
                continue;
            }
            let current_site = current.site;
            let neighbors = self.neighbors(cell, current_site);
            let mut satisfied = false;

            for &(neighbor_cell, neighbor_site) in &neighbors {
                let neighbor_index = self.identity_at(state, neighbor_cell, neighbor_site)?;
                let neighbor_id = &self.field.surface_biomes[neighbor_index].id;
                if biome.require_near.iter().any(|required| required == neighbor_id) {
                    satisfied = true;
                    break;
                }
            }
            if satisfied {
                continue;
            }

            for (neighbor_cell, neighbor_site) in neighbors {
                let domain = self.candidate_order(neighbor_cell, neighbor_site)?;
                let can_satisfy = domain.iter().any(|candidate_index| {
                    let id = &self.field.surface_biomes[*candidate_index].id;
                    biome.require_near.iter().any(|required| required == id)
                });
                if can_satisfy {
                    self.ensure_cell(state, neighbor_cell)?;
                }
            }
            return Ok(Some(cell));
        }
        Ok(None)
    }

    fn boundary_cycle_violation(&self, state: &SolverState) -> Option<Vec<IVec2>> {
        let mut remaining = state.cells.keys().copied().collect::<HashSet<_>>();

        while !remaining.is_empty() {
            let start = remaining
                .iter()
                .copied()
                .min_by_key(|cell| (cell.y, cell.x))
                .expect("non-empty boundary component set must have a start");
            remaining.remove(&start);
            let mut component = vec![start];
            let mut queue = VecDeque::from([start]);

            while let Some(cell) = queue.pop_front() {
                let current = state.cells.get(&cell).expect("solver cell must exist");
                for (neighbor_cell, _) in self.neighbors(cell, current.site) {
                    if !remaining.contains(&neighbor_cell) {
                        continue;
                    }
                    let neighbor = state
                        .cells
                        .get(&neighbor_cell)
                        .expect("remaining solver neighbor must exist");
                    if neighbor.biome_index() == current.biome_index() {
                        continue;
                    }
                    remaining.remove(&neighbor_cell);
                    component.push(neighbor_cell);
                    queue.push_back(neighbor_cell);
                }
            }

            if component.len() <= 1 {
                continue;
            }
            component.sort_by_key(|cell| (cell.y, cell.x));
            let sampled = component
                .iter()
                .map(|cell| {
                    let current = state.cells.get(cell).expect("solver cell must exist");
                    (*cell, current.site, 0.0, Some(current.biome_index()))
                })
                .collect::<Vec<_>>();

            let Err(failure) = fitting::fit_surface_site_weights_detailed(
                &sampled,
                &self.field.surface_biomes,
                self.field.surface_site_spacing,
                self.field.seed,
            ) else {
                continue;
            };

            let mut repairable = failure
                .cells
                .into_iter()
                .filter(|cell| {
                    state
                        .cells
                        .get(cell)
                        .is_some_and(|current| current.domain.len() > 1)
                })
                .collect::<Vec<_>>();
            repairable.sort_unstable_by_key(|cell| (cell.y, cell.x));
            repairable.dedup();
            return Some(repairable);
        }
        None
    }

    fn repair_branches(&self, state: &SolverState, violation: Violation) -> Vec<SolverState> {
        match violation {
            Violation::Pair { left, right } => self.branch_on_mrv(state, vec![left, right]),
            Violation::Oversize { cells } | Violation::BoundaryCycle { cells } => {
                self.branch_on_mrv(state, cells)
            }
            Violation::RequireNear { cell } => self.require_near_branches(state, cell),
        }
    }

    fn branch_on_mrv(&self, state: &SolverState, mut cells: Vec<IVec2>) -> Vec<SolverState> {
        cells.sort_unstable_by_key(|cell| (cell.y, cell.x));
        cells.dedup();
        let pivot = cells
            .into_iter()
            .filter(|cell| state.cells.get(cell).is_some_and(|current| current.domain.len() > 1))
            .min_by_key(|cell| {
                let current = &state.cells[cell];
                (
                    current.domain.len(),
                    std::cmp::Reverse(region_claim_hash(
                        *cell,
                        current.biome_index(),
                        self.field.seed,
                    )),
                    cell.y,
                    cell.x,
                )
            });
        let Some(pivot) = pivot else {
            return Vec::new();
        };

        let domain = state.cells[&pivot].domain.clone();
        domain
            .into_iter()
            .map(|candidate| {
                let mut branch = state.clone();
                branch
                    .cells
                    .get_mut(&pivot)
                    .expect("branch pivot must exist")
                    .domain = vec![candidate];
                branch
            })
            .collect()
    }

    fn require_near_branches(&self, state: &SolverState, cell: IVec2) -> Vec<SolverState> {
        let Some(current) = state.cells.get(&cell) else {
            return Vec::new();
        };
        let biome = &self.field.surface_biomes[current.biome_index()];
        let mut candidates = vec![cell];

        for (neighbor_cell, _) in self.neighbors(cell, current.site) {
            let Some(neighbor) = state.cells.get(&neighbor_cell) else {
                continue;
            };
            if neighbor.domain.iter().any(|candidate_index| {
                let id = &self.field.surface_biomes[*candidate_index].id;
                biome.require_near.iter().any(|required| required == id)
            }) {
                candidates.push(neighbor_cell);
            }
        }
        self.branch_on_mrv(state, candidates)
    }
}
