use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use super::{
    SurfaceSelectionContext, authored_pair_conflicts, cell_hash, climate_weight,
    distribution_strength, fitting, raw_surface_biome_index, region_claim_hash,
    surface_site_position, surface_sites_share_border,
};

const FIT_EPSILON: f32 = 0.001;

#[derive(Clone, Copy)]
struct RawComponentNode {
    cell: IVec2,
    site: Vec2,
}

#[derive(Clone, Copy)]
struct ExternalNeighbor {
    cell: IVec2,
    site: Vec2,
    inside_site: Vec2,
    biome_index: usize,
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

    fn span(self) -> Vec2 {
        self.max - self.min
    }

    fn fits(self, limit: Vec2) -> bool {
        let span = self.span();
        span.x <= limit.x + FIT_EPSILON && span.y <= limit.y + FIT_EPSILON
    }
}

pub(super) fn raw_conflict_left_wins(
    left_cell: IVec2,
    left_site: Vec2,
    left_index: usize,
    right_cell: IVec2,
    right_site: Vec2,
    right_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let left_can_yield = fitting::cell_has_alternative(left_cell, left_site, left_index, context)
        || component_takeover_candidate(left_cell, left_site, left_index, context).is_some();
    let right_can_yield = fitting::cell_has_alternative(right_cell, right_site, right_index, context)
        || component_takeover_candidate(right_cell, right_site, right_index, context).is_some();

    match (left_can_yield, right_can_yield) {
        (false, true) => true,
        (true, false) => false,
        _ => fitting::raw_conflict_left_wins(
            left_cell,
            left_site,
            left_index,
            right_cell,
            right_site,
            right_index,
            context,
        ),
    }
}

/// Last-resort fitted continuation for a raw component that has no legal local
/// candidate. The previous resolver flood-filled the complete raw component,
/// which made locate/world probes walk arbitrarily large regions. This version
/// derives a hard traversal bound from the largest authored max-size slack of
/// every biome that could legally take the component over.
pub(super) fn component_takeover_candidate(
    cell: IVec2,
    site: Vec2,
    raw_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> Option<usize> {
    if context.ocean_surface_index == Some(raw_index) {
        return None;
    }

    let traversal_limit = maximum_takeover_span(raw_index, context)?;
    let component = collect_raw_component(cell, site, raw_index, traversal_limit, context)?;
    let external = collect_external_neighbors(&component, raw_index, context);
    if external.is_empty() {
        return None;
    }

    let canonical_cell = component
        .iter()
        .map(|node| node.cell)
        .min_by_key(|cell| (cell.y, cell.x))
        .unwrap_or(cell);

    let touching = external
        .iter()
        .map(|neighbor| neighbor.biome_index)
        .collect::<HashSet<_>>();

    let mut candidates = context
        .biomes
        .iter()
        .enumerate()
        .filter_map(|(candidate_index, _)| {
            if !takeover_candidate_enabled(candidate_index, raw_index, context) {
                return None;
            }

            // A biome already touching the failed component may continue across
            // its climate edge. Otherwise, only consider a biome whose authored
            // climate/distribution supports this site.
            let is_touching = touching.contains(&candidate_index);
            if !is_touching && !candidate_supported_at_site(candidate_index, site, context) {
                return None;
            }

            Some((candidate_index, is_touching))
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|(candidate_index, is_touching)| {
        (
            !*is_touching,
            region_claim_hash(canonical_cell, *candidate_index, context.seed),
            *candidate_index,
        )
    });

    candidates
        .into_iter()
        .map(|(candidate_index, _)| candidate_index)
        .find(|candidate_index| {
            takeover_boundary_allows(*candidate_index, &external, context)
                && takeover_size_allows(*candidate_index, &component, &external, context)
        })
}

fn takeover_candidate_enabled(
    candidate_index: usize,
    raw_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    if candidate_index == raw_index || context.biomes[candidate_index].weight <= 0.0 {
        return false;
    }
    if !context.spawn_oceans && Some(candidate_index) == context.ocean_surface_index {
        return false;
    }

    // Ocean owns its authored macro core before the resolver runs. Letting it
    // act as an unlimited takeover fallback would remove the finite traversal
    // bound and could leak ocean identity inland.
    if Some(candidate_index) == context.ocean_surface_index {
        return false;
    }

    // Exclusive biomes are raw-only. They may keep their own raw sites, but
    // may never spread as a fallback when another raw biome has to yield.
    context.biomes[candidate_index]
        .exclusive_neighbor_group
        .is_none()
}

fn candidate_supported_at_site(
    candidate_index: usize,
    site: Vec2,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let candidate = &context.biomes[candidate_index];
    if climate_weight(context.climate_field.sample(site), candidate.climate) <= 0.0 {
        return false;
    }

    candidate
        .distributions
        .iter()
        .copied()
        .map(|distribution| {
            distribution_strength(distribution, site, context.seed, candidate.id.as_str())
        })
        .fold(0.0_f32, f32::max)
        > 0.0
}

fn maximum_takeover_span(
    raw_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> Option<Vec2> {
    context
        .biomes
        .iter()
        .enumerate()
        .filter(|(candidate_index, _)| {
            takeover_candidate_enabled(*candidate_index, raw_index, context)
        })
        .map(|(_, candidate)| candidate_center_span_limit(candidate))
        .reduce(Vec2::max)
}

fn candidate_center_span_limit(candidate: &crate::world::biome_field::BiomeFieldEntry) -> Vec2 {
    Vec2::new(
        ((candidate.size.x.max - candidate.size.x.min) * 2.0).max(0.0),
        ((candidate.size.z.max - candidate.size.z.min) * 2.0).max(0.0),
    )
}

fn collect_raw_component(
    cell: IVec2,
    site: Vec2,
    raw_index: usize,
    traversal_limit: Vec2,
    context: &SurfaceSelectionContext<'_>,
) -> Option<Vec<RawComponentNode>> {
    let mut nodes = Vec::new();
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    let mut bounds = SiteBounds::from_site(site);

    queue.push_back((cell, site));
    seen.insert(cell);

    while let Some((current_cell, current_site)) = queue.pop_front() {
        nodes.push(RawComponentNode {
            cell: current_cell,
            site: current_site,
        });

        for z in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
            for x in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let neighbor_cell = current_cell + offset;
                if seen.contains(&neighbor_cell) {
                    continue;
                }
                let neighbor_site =
                    surface_site_position(neighbor_cell, context.spacing, context.seed);
                if !surface_sites_share_border(
                    current_cell,
                    current_site,
                    neighbor_cell,
                    neighbor_site,
                    context.spacing,
                    context.seed,
                ) {
                    continue;
                }
                if raw_index_at(neighbor_cell, neighbor_site, context) != raw_index {
                    continue;
                }

                // Every legal whole-component takeover has a finite authored
                // center-span budget. Once the raw component exceeds the largest
                // possible budget, no takeover candidate can fit it, so stop
                // immediately instead of walking the rest of the component.
                let mut next_bounds = bounds;
                next_bounds.include(neighbor_site);
                if !next_bounds.fits(traversal_limit) {
                    return None;
                }
                bounds = next_bounds;

                seen.insert(neighbor_cell);
                queue.push_back((neighbor_cell, neighbor_site));
            }
        }
    }

    Some(nodes)
}

fn collect_external_neighbors(
    component: &[RawComponentNode],
    raw_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> Vec<ExternalNeighbor> {
    let component_cells = component
        .iter()
        .map(|node| node.cell)
        .collect::<HashSet<_>>();
    let mut external = HashMap::<IVec2, ExternalNeighbor>::new();

    for node in component {
        for z in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
            for x in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let neighbor_cell = node.cell + offset;
                if component_cells.contains(&neighbor_cell) {
                    continue;
                }
                let neighbor_site =
                    surface_site_position(neighbor_cell, context.spacing, context.seed);
                if !surface_sites_share_border(
                    node.cell,
                    node.site,
                    neighbor_cell,
                    neighbor_site,
                    context.spacing,
                    context.seed,
                ) {
                    continue;
                }
                let biome_index = raw_index_at(neighbor_cell, neighbor_site, context);
                if biome_index == raw_index {
                    continue;
                }
                external.entry(neighbor_cell).or_insert(ExternalNeighbor {
                    cell: neighbor_cell,
                    site: neighbor_site,
                    inside_site: node.site,
                    biome_index,
                });
            }
        }
    }

    external.into_values().collect()
}

fn takeover_boundary_allows(
    candidate_index: usize,
    external: &[ExternalNeighbor],
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let candidate = &context.biomes[candidate_index];
    let mut required_neighbor_found = candidate.require_near.is_empty();

    for neighbor in external {
        let neighbor_biome = &context.biomes[neighbor.biome_index];
        if neighbor.biome_index != candidate_index
            && (authored_pair_conflicts(candidate, neighbor_biome)
                || fitting::boundary_fit_interval(
                    candidate,
                    neighbor_biome,
                    neighbor.inside_site,
                    neighbor.site,
                )
                .is_none())
        {
            return false;
        }

        if !required_neighbor_found
            && candidate
                .require_near
                .iter()
                .any(|id| id == &neighbor_biome.id)
        {
            required_neighbor_found = true;
        }
    }

    required_neighbor_found
}

fn takeover_size_allows(
    candidate_index: usize,
    component: &[RawComponentNode],
    external: &[ExternalNeighbor],
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let center_span_limit = candidate_center_span_limit(&context.biomes[candidate_index]);
    let Some(first) = component.first() else {
        return false;
    };
    let mut bounds = SiteBounds::from_site(first.site);
    for node in &component[1..] {
        bounds.include(node.site);
    }
    if !bounds.fits(center_span_limit) {
        return false;
    }

    // A takeover joins any touching raw component of the replacement biome.
    // Include those connected sites in the max-size check. The walk is itself
    // bounded by the candidate's authored span, so it cannot recreate the old
    // unbounded locate regression.
    let mut queue = VecDeque::new();
    let mut seen = component
        .iter()
        .map(|node| node.cell)
        .collect::<HashSet<_>>();
    for neighbor in external
        .iter()
        .filter(|neighbor| neighbor.biome_index == candidate_index)
    {
        if seen.insert(neighbor.cell) {
            queue.push_back((neighbor.cell, neighbor.site));
        }
    }

    while let Some((cell, site)) = queue.pop_front() {
        bounds.include(site);
        if !bounds.fits(center_span_limit) {
            return false;
        }

        for z in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
            for x in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let neighbor_cell = cell + offset;
                if seen.contains(&neighbor_cell) {
                    continue;
                }
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
                if raw_index_at(neighbor_cell, neighbor_site, context) != candidate_index {
                    continue;
                }

                let mut next_bounds = bounds;
                next_bounds.include(neighbor_site);
                if !next_bounds.fits(center_span_limit) {
                    return false;
                }
                seen.insert(neighbor_cell);
                queue.push_back((neighbor_cell, neighbor_site));
            }
        }
    }

    true
}

fn raw_index_at(
    cell: IVec2,
    site: Vec2,
    context: &SurfaceSelectionContext<'_>,
) -> usize {
    raw_surface_biome_index(
        cell,
        site,
        context.climate_field.sample(site),
        cell_hash(cell, context.seed),
        context.biomes,
        context.seed,
        context.spawn_oceans,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_bounds_accept_exact_authored_span() {
        let mut bounds = SiteBounds::from_site(Vec2::ZERO);
        bounds.include(Vec2::new(600.0, 440.0));
        assert!(bounds.fits(Vec2::new(600.0, 440.0)));
    }

    #[test]
    fn site_bounds_reject_span_past_authored_maximum() {
        let mut bounds = SiteBounds::from_site(Vec2::ZERO);
        bounds.include(Vec2::new(600.01, 440.0));
        assert!(!bounds.fits(Vec2::new(600.0, 440.0)));
    }
}
