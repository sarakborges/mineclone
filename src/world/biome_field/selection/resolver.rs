use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use super::{
    SurfaceSelectionContext, authored_pair_conflicts, cell_hash, climate_weight,
    distribution_strength, fitting, raw_surface_biome_index, region_claim_hash,
    surface_site_position, surface_sites_share_border,
};
use crate::world::biome_field::BiomeField;

const FIT_EPSILON: f32 = 0.001;

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

#[derive(Clone)]
enum Violation {
    Pair { left: IVec2, right: IVec2 },
    Oversize { cells: Vec<IVec2> },
    RequireNear { cell: IVec2 },
    BoundaryCycle { cells: Vec<IVec2> },
}

struct RecursiveSurfaceSolver<'a> {
    field: &'a BiomeField,
}

/// Resolves a canonical group of surface sites as one constraint problem.
///
/// The old path selected each site independently and only attempted a bounded
/// whole-component takeover after local fitting had already failed. That meant
/// a valid solution could require changing A -> B -> C while the resolver gave
/// up at A. This solver instead expands only when a constraint reaches an
/// unresolved neighbor and recursively backtracks assignments until every
/// reached min/max and authored-adjacency constraint is simultaneously valid.
/// There is deliberately no propagation-depth or component-size cutoff.
pub(in crate::world::biome_field) fn resolve_surface_sites(
    field: &BiomeField,
    requested_cells: &[IVec2],
) -> Result<Vec<(IVec2, Vec2, usize)>, String> {
    let solver = RecursiveSurfaceSolver { field };
    let mut state = SolverState::default();
    for &cell in requested_cells {
        solver.ensure_cell(&mut state, cell)?;
    }

    let solved = solver.solve(state).ok_or_else(|| {
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
    Ok(resolved)
}

impl RecursiveSurfaceSolver<'_> {
    fn solve(&self, mut state: SolverState) -> Option<SolverState> {
        if self.expand_pair_conflicts(&mut state).is_err() {
            return None;
        }

        let violation = if let Some((left, right)) = self.first_pair_violation(&state) {
            Some(Violation::Pair { left, right })
        } else if let Ok(Some(cells)) = self.first_oversize_violation(&mut state) {
            Some(Violation::Oversize { cells })
        } else if let Some(cell) = self.first_require_near_violation(&state) {
            Some(Violation::RequireNear { cell })
        } else {
            self.boundary_cycle_violation(&state)
                .map(|cells| Violation::BoundaryCycle { cells })
        };

        let Some(violation) = violation else {
            return Some(state);
        };

        for branch in self.repair_branches(&state, violation) {
            if let Some(solved) = self.solve(branch) {
                return Some(solved);
            }
        }
        None
    }

    fn ensure_cell(&self, state: &mut SolverState, cell: IVec2) -> Result<(), String> {
        if state.cells.contains_key(&cell) {
            return Ok(());
        }
        let site = surface_site_position(cell, self.field.surface_site_spacing, self.field.seed);
        let domain = self.candidate_order(cell, site)?;
        state.cells.insert(cell, CellState { site, domain });
        Ok(())
    }

    fn candidate_order(&self, cell: IVec2, site: Vec2) -> Result<Vec<usize>, String> {
        let climate = self.field.climate.sample(site);

        // The authored ocean core remains authoritative. It is a macro mask,
        // not a candidate that recursive fitting is allowed to erase.
        if let Some(ocean_index) = self.field.ocean_surface_index
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
                return Ok(vec![ocean_index]);
            }
        }

        let candidates = self
            .field
            .surface_weighted_candidates(cell, site, climate, cell_hash(cell, self.field.seed))
            .into_iter()
            .map(|candidate| candidate.index)
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            return Err(format!(
                "surface biome site {cell:?} has no biome compatible with climate/distribution constraints"
            ));
        }
        Ok(candidates)
    }

    fn raw_identity(&self, cell: IVec2, site: Vec2) -> Result<usize, String> {
        self.candidate_order(cell, site)
            .map(|domain| domain[0])
    }

    fn identity_at(&self, state: &SolverState, cell: IVec2, site: Vec2) -> Result<usize, String> {
        state
            .cells
            .get(&cell)
            .map(CellState::biome_index)
            .map_or_else(|| self.raw_identity(cell, site), Ok)
    }

    fn neighbors(&self, cell: IVec2, site: Vec2) -> Vec<(IVec2, Vec2)> {
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
        neighbors.sort_by_key(|(cell, _)| (cell.y, cell.x));
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

    /// Any raw neighbor that conflicts with an already reached assignment must
    /// become part of the same solve. Compatible raw neighbors remain fixed
    /// boundary conditions, keeping ordinary world probes local.
    fn expand_pair_conflicts(&self, state: &mut SolverState) -> Result<(), String> {
        loop {
            let mut cells = state.cells.keys().copied().collect::<Vec<_>>();
            cells.sort_by_key(|cell| (cell.y, cell.x));
            let mut expansion = None;

            'scan: for cell in cells {
                let left_state = state.cells.get(&cell).expect("solver cell must exist");
                let left_site = left_state.site;
                let left_index = left_state.biome_index();
                for (neighbor_cell, neighbor_site) in self.neighbors(cell, left_site) {
                    if state.cells.contains_key(&neighbor_cell) {
                        continue;
                    }
                    let right_index = self.raw_identity(neighbor_cell, neighbor_site)?;
                    if !self.pair_allows(left_index, left_site, right_index, neighbor_site) {
                        expansion = Some(neighbor_cell);
                        break 'scan;
                    }
                }
            }

            let Some(cell) = expansion else {
                return Ok(());
            };
            self.ensure_cell(state, cell)?;
        }
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

    /// Walks same-biome identity only until the authored maximum is proven to
    /// be exceeded. Unlike the removed takeover flood-fill, a huge raw region
    /// never has to be collected in full before the solver can start cutting
    /// it: the first oversized prefix becomes a recursive repair problem.
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
            let mut visited = Vec::new();

            while let Some((cell, site)) = queue.pop_front() {
                visited.push(cell);
                bounds.include(site);
                if bounds.exceeds(center_span_limit) {
                    for &visited_cell in &visited {
                        self.ensure_cell(state, visited_cell)?;
                    }
                    visited.sort_by_key(|cell| (cell.y, cell.x));
                    return Ok(Some(visited));
                }

                for (neighbor_cell, neighbor_site) in self.neighbors(cell, site) {
                    if seen.contains(&neighbor_cell) {
                        continue;
                    }
                    let neighbor_index = self.identity_at(state, neighbor_cell, neighbor_site)?;
                    if neighbor_index != biome_index {
                        continue;
                    }
                    seen.insert(neighbor_cell);
                    queue.push_back((neighbor_cell, neighbor_site));
                }
            }

            for cell in visited {
                if state.cells.contains_key(&cell) {
                    checked.insert(cell);
                }
            }
        }
        Ok(None)
    }

    fn first_require_near_violation(&self, state: &SolverState) -> Option<IVec2> {
        let mut cells = state.cells.keys().copied().collect::<Vec<_>>();
        cells.sort_by_key(|cell| (cell.y, cell.x));

        for cell in cells {
            let current = state.cells.get(&cell).expect("solver cell must exist");
            let biome = &self.field.surface_biomes[current.biome_index()];
            if biome.require_near.is_empty() {
                continue;
            }

            let found = self.neighbors(cell, current.site).into_iter().any(
                |(neighbor_cell, neighbor_site)| {
                    self.identity_at(state, neighbor_cell, neighbor_site)
                        .ok()
                        .is_some_and(|neighbor_index| {
                            let neighbor_id = &self.field.surface_biomes[neighbor_index].id;
                            biome.require_near.iter().any(|id| id == neighbor_id)
                        })
                },
            );
            if !found {
                return Some(cell);
            }
        }
        None
    }

    fn boundary_cycle_violation(&self, state: &SolverState) -> Option<Vec<IVec2>> {
        let mut cells = state.cells.keys().copied().collect::<Vec<_>>();
        cells.sort_by_key(|cell| (cell.y, cell.x));
        if cells.len() <= 1 {
            return None;
        }

        let sampled = cells
            .iter()
            .map(|cell| {
                let current = state.cells.get(cell).expect("solver cell must exist");
                (*cell, current.site, 0.0, Some(current.biome_index()))
            })
            .collect::<Vec<_>>();

        if fitting::fit_surface_site_weights(
            &sampled,
            &self.field.surface_biomes,
            self.field.surface_site_spacing,
            self.field.seed,
        )
        .is_ok()
        {
            return None;
        }

        let mut repairable = cells
            .into_iter()
            .filter(|cell| {
                state
                    .cells
                    .get(cell)
                    .is_some_and(|current| current.domain.len() > 1)
            })
            .collect::<Vec<_>>();
        repairable.sort_by_key(|cell| {
            (
                std::cmp::Reverse(region_claim_hash(
                    *cell,
                    state.cells[cell].biome_index(),
                    self.field.seed,
                )),
                cell.y,
                cell.x,
            )
        });
        (!repairable.is_empty()).then_some(repairable)
    }

    fn repair_branches(&self, state: &SolverState, violation: Violation) -> Vec<SolverState> {
        match violation {
            Violation::Pair { left, right } => {
                let mut cells = vec![left, right];
                cells.sort_by_key(|cell| {
                    std::cmp::Reverse(region_claim_hash(
                        *cell,
                        state.cells[cell].biome_index(),
                        self.field.seed,
                    ))
                });
                self.drop_current_branches(state, cells)
            }
            Violation::Oversize { mut cells } | Violation::BoundaryCycle { mut cells } => {
                cells.sort_by_key(|cell| {
                    (
                        std::cmp::Reverse(region_claim_hash(
                            *cell,
                            state.cells[cell].biome_index(),
                            self.field.seed,
                        )),
                        cell.y,
                        cell.x,
                    )
                });
                self.drop_current_branches(state, cells)
            }
            Violation::RequireNear { cell } => self.require_near_branches(state, cell),
        }
    }

    fn drop_current_branches(
        &self,
        state: &SolverState,
        cells: Vec<IVec2>,
    ) -> Vec<SolverState> {
        let mut branches = Vec::new();
        for cell in cells {
            let Some(current) = state.cells.get(&cell) else {
                continue;
            };
            if current.domain.len() <= 1 {
                continue;
            }
            let mut branch = state.clone();
            branch
                .cells
                .get_mut(&cell)
                .expect("branch cell must exist")
                .domain
                .remove(0);
            branches.push(branch);
        }
        branches
    }

    fn require_near_branches(&self, state: &SolverState, cell: IVec2) -> Vec<SolverState> {
        let Some(current) = state.cells.get(&cell) else {
            return Vec::new();
        };
        let biome = &self.field.surface_biomes[current.biome_index()];
        let mut branches = self.drop_current_branches(state, vec![cell]);

        for (neighbor_cell, _) in self.neighbors(cell, current.site) {
            let Some(neighbor) = state.cells.get(&neighbor_cell) else {
                continue;
            };
            let Some(position) = neighbor.domain.iter().position(|candidate_index| {
                let id = &self.field.surface_biomes[*candidate_index].id;
                biome.require_near.iter().any(|required| required == id)
            }) else {
                continue;
            };
            if position == 0 {
                continue;
            }

            let mut branch = state.clone();
            branch
                .cells
                .get_mut(&neighbor_cell)
                .expect("branch neighbor must exist")
                .domain
                .drain(..position);
            branches.push(branch);
        }
        branches
    }
}

/// Kept for the legacy single-site selector while callers migrate to the
/// recursive group resolver. Conflict priority itself stays deterministic.
pub(super) fn raw_conflict_left_wins(
    left_cell: IVec2,
    left_site: Vec2,
    left_index: usize,
    right_cell: IVec2,
    right_site: Vec2,
    right_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    fitting::raw_conflict_left_wins(
        left_cell,
        left_site,
        left_index,
        right_cell,
        right_site,
        right_index,
        context,
    )
}

/// Compatibility shim for the old selector. Surface sampling no longer relies
/// on this bounded takeover path; it resolves the whole reached constraint
/// graph through `resolve_surface_sites` before anything is cached.
pub(super) fn component_takeover_candidate(
    cell: IVec2,
    site: Vec2,
    raw_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> Option<usize> {
    let climate = context.climate_field.sample(site);
    let mut candidates = context
        .biomes
        .iter()
        .enumerate()
        .filter(|(candidate_index, biome)| {
            *candidate_index != raw_index
                && biome.weight > 0.0
                && (context.spawn_oceans || Some(*candidate_index) != context.ocean_surface_index)
        })
        .filter(|(_, biome)| {
            biome
                .distributions
                .iter()
                .copied()
                .map(|distribution| {
                    distribution_strength(distribution, site, context.seed, biome.id.as_str())
                })
                .fold(0.0_f32, f32::max)
                > 0.0
                && climate_weight(climate, biome.climate) > 0.0
        })
        .map(|(candidate_index, _)| candidate_index)
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate_index| {
        region_claim_hash(cell, *candidate_index, context.seed)
    });

    candidates.into_iter().find(|candidate_index| {
        let candidate = &context.biomes[*candidate_index];
        let mut required_neighbor_found = candidate.require_near.is_empty();

        for z in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
            for x in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let neighbor_cell = cell + offset;
                let neighbor_site =
                    surface_site_position(neighbor_cell, context.spacing, context.seed);
                if !surface_sites_share_border(
                    cell,
                    site,
                    neighbor_cell,
                    neighbor_site,
                    context.spacing,
                    context.seed,
                ) {
                    continue;
                }
                let neighbor_index = raw_surface_biome_index(
                    neighbor_cell,
                    neighbor_site,
                    context.climate_field.sample(neighbor_site),
                    cell_hash(neighbor_cell, context.seed),
                    context.biomes,
                    context.seed,
                    context.spawn_oceans,
                );
                let neighbor = &context.biomes[neighbor_index];
                if neighbor_index != *candidate_index
                    && (authored_pair_conflicts(candidate, neighbor)
                        || fitting::boundary_fit_interval(
                            candidate,
                            neighbor,
                            site,
                            neighbor_site,
                        )
                        .is_none())
                {
                    return false;
                }
                if !required_neighbor_found
                    && candidate.require_near.iter().any(|id| id == &neighbor.id)
                {
                    required_neighbor_found = true;
                }
            }
        }

        if !required_neighbor_found {
            return false;
        }

        let fallback_context = SurfaceSelectionContext {
            cell,
            site,
            spacing: context.spacing,
            seed: context.seed,
            biomes: context.biomes,
            spawn_oceans: context.spawn_oceans,
            ocean_surface_index: context.ocean_surface_index,
            climate_field: context.climate_field,
        };
        fitting::surface_size_allows(*candidate_index, &fallback_context)
    })
}
