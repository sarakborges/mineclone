use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use super::{
    SurfaceSelectionContext, authored_pair_conflicts, cell_hash, fitting, raw_surface_biome_index,
    region_claim_hash, surface_site_position, surface_sites_share_border,
};

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
        _ => {
            let left = (
                adjacency_claim_hash(left_cell, left_index, context.seed),
                left_cell.y,
                left_cell.x,
                left_index,
            );
            let right = (
                adjacency_claim_hash(right_cell, right_index, context.seed),
                right_cell.y,
                right_cell.x,
                right_index,
            );
            left <= right
        }
    }
}

/// Last-resort fitted continuation. If a raw component has no local legal
/// fallback, the whole component may yield to one of the biomes touching its
/// boundary. The replacement is validated against the component's final
/// external boundary instead of a stale raw-neighbor snapshot.
pub(super) fn component_takeover_candidate(
    cell: IVec2,
    site: Vec2,
    raw_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> Option<usize> {
    if context.ocean_surface_index == Some(raw_index) {
        return None;
    }

    let component = collect_raw_component(cell, site, raw_index, context);
    let external = collect_external_neighbors(&component, raw_index, context);
    if external.is_empty() {
        return None;
    }

    let canonical_cell = component
        .iter()
        .map(|node| node.cell)
        .min_by_key(|cell| (cell.y, cell.x))
        .unwrap_or(cell);

    let mut candidates = external
        .iter()
        .map(|neighbor| neighbor.biome_index)
        .filter(|candidate_index| {
            *candidate_index != raw_index
                && context.biomes[*candidate_index].weight > 0.0
                && (context.spawn_oceans
                    || Some(*candidate_index) != context.ocean_surface_index)
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    candidates.sort_by_key(|candidate_index| {
        (
            region_claim_hash(canonical_cell, *candidate_index, context.seed),
            *candidate_index,
        )
    });

    candidates.into_iter().find(|candidate_index| {
        takeover_boundary_allows(*candidate_index, &external, context)
            && takeover_size_allows(*candidate_index, &component, &external, context)
    })
}

fn collect_raw_component(
    cell: IVec2,
    site: Vec2,
    raw_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> Vec<RawComponentNode> {
    let mut nodes = Vec::new();
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
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

                seen.insert(neighbor_cell);
                queue.push_back((neighbor_cell, neighbor_site));
            }
        }
    }

    nodes
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
    if context.ocean_surface_index == Some(candidate_index) {
        return true;
    }

    let candidate = &context.biomes[candidate_index];
    let center_span_limit = Vec2::new(
        ((candidate.size.x.max - candidate.size.x.min) * 2.0).max(0.0),
        ((candidate.size.z.max - candidate.size.z.min) * 2.0).max(0.0),
    );
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for node in component {
        min = min.min(node.site);
        max = max.max(node.site);
    }

    for neighbor in external
        .iter()
        .filter(|neighbor| neighbor.biome_index == candidate_index)
    {
        min = min.min(neighbor.site);
        max = max.max(neighbor.site);
    }

    let span = max - min;
    span.x <= center_span_limit.x + 0.001 && span.y <= center_span_limit.y + 0.001
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

fn adjacency_claim_hash(cell: IVec2, biome_index: usize, seed: u64) -> u64 {
    region_claim_hash(cell, biome_index, seed) ^ 0x6a09_e667_f3bc_c909
}
