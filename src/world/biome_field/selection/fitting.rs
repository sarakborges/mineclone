use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use super::{
    SurfaceSelectionContext, cell_hash, climate_weight, distribution_strength, raw_surface_biome_index,
    region_claim_hash, surface_sites_share_border, surface_site_position,
};
use crate::world::biome_field::BiomeFieldEntry;

const FIT_EPSILON: f32 = 0.001;

#[derive(Clone, Copy, Debug)]
struct ComponentNode {
    cell: IVec2,
    site: Vec2,
    removable: bool,
    claim: u64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct BoundaryFitInterval {
    pub(super) min_from_left: f32,
    pub(super) max_from_left: f32,
}

/// `size.min` is guaranteed by the authored site lattice. `size.max` is fitted
/// by spending only the slack between min and max. Raw same-biome components
/// are iteratively cut until every surviving component can fit inside the
/// authored maximum while retaining its minimum radius on both sides.
pub(super) fn surface_size_allows(
    candidate_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    // The ocean core is an authored macro mask and is intentionally allowed to
    // form one large body. It is selected authoritatively before this fitter.
    if context.ocean_surface_index == Some(candidate_index) {
        return true;
    }

    let nodes = collect_raw_component(candidate_index, context);
    if nodes.len() <= 1 {
        return true;
    }

    let neighbors = component_neighbors(&nodes, context);
    let candidate = &context.biomes[candidate_index];
    let center_span_limit = Vec2::new(
        ((candidate.size.x.max - candidate.size.x.min) * 2.0).max(0.0),
        ((candidate.size.z.max - candidate.size.z.min) * 2.0).max(0.0),
    );

    fit_component_mask(&nodes, &neighbors, center_span_limit)
        .map(|active| active[0])
        .unwrap_or(false)
}

/// When two authored adjacency rules conflict, exactly one raw side yields.
/// A side with no climate/distribution-compatible alternative is protected;
/// otherwise a stable hash chooses the winner. Calling this with the pair
/// reversed therefore produces the opposite answer, so both sites cannot
/// reject each other merely because they were evaluated independently.
pub(super) fn raw_conflict_left_wins(
    left_cell: IVec2,
    left_site: Vec2,
    left_index: usize,
    right_cell: IVec2,
    right_site: Vec2,
    right_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let left_can_yield = cell_has_alternative(left_cell, left_site, left_index, context);
    let right_can_yield = cell_has_alternative(right_cell, right_site, right_index, context);

    match (left_can_yield, right_can_yield) {
        (false, true) => true,
        (true, false) => false,
        _ => {
            let left = adjacency_claim_hash(left_cell, left_index, context.seed);
            let right = adjacency_claim_hash(right_cell, right_index, context.seed);
            left <= right
        }
    }
}

pub(super) fn cell_has_alternative(
    cell: IVec2,
    site: Vec2,
    candidate_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> bool {
    let climate = context.climate_field.sample(site);

    context
        .biomes
        .iter()
        .enumerate()
        .filter(|(index, biome)| {
            *index != candidate_index
                && biome.weight > 0.0
                && (context.spawn_oceans || Some(*index) != context.ocean_surface_index)
        })
        .any(|(_, biome)| {
            let distribution = biome
                .distributions
                .iter()
                .copied()
                .map(|distribution| {
                    distribution_strength(distribution, site, context.seed, biome.id.as_str())
                })
                .fold(0.0_f32, f32::max);
            distribution > 0.0 && climate_weight(climate, biome.climate) > 0.0
        })
}

fn collect_raw_component(
    candidate_index: usize,
    context: &SurfaceSelectionContext<'_>,
) -> Vec<ComponentNode> {
    let mut nodes = Vec::new();
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();

    queue.push_back((context.cell, context.site));
    seen.insert(context.cell);

    while let Some((cell, site)) = queue.pop_front() {
        nodes.push(ComponentNode {
            cell,
            site,
            removable: cell_has_alternative(cell, site, candidate_index, context),
            claim: region_claim_hash(cell, candidate_index, context.seed),
        });

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

                let neighbor_index = raw_surface_biome_index(
                    neighbor_cell,
                    neighbor_site,
                    context.climate_field.sample(neighbor_site),
                    cell_hash(neighbor_cell, context.seed),
                    context.biomes,
                    context.seed,
                    context.spawn_oceans,
                );
                if neighbor_index != candidate_index {
                    continue;
                }

                seen.insert(neighbor_cell);
                queue.push_back((neighbor_cell, neighbor_site));
            }
        }
    }

    nodes
}

fn component_neighbors(
    nodes: &[ComponentNode],
    context: &SurfaceSelectionContext<'_>,
) -> Vec<Vec<usize>> {
    let by_cell = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.cell, index))
        .collect::<HashMap<_, _>>();
    let mut neighbors = vec![Vec::new(); nodes.len()];

    for (index, node) in nodes.iter().enumerate() {
        for z in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
            for x in -super::SITE_SEARCH_RADIUS..=super::SITE_SEARCH_RADIUS {
                let offset = IVec2::new(x, z);
                if offset == IVec2::ZERO {
                    continue;
                }
                let Some(&neighbor_index) = by_cell.get(&(node.cell + offset)) else {
                    continue;
                };
                if neighbor_index <= index {
                    continue;
                }
                let neighbor = nodes[neighbor_index];
                if !surface_sites_share_border(
                    node.cell,
                    node.site,
                    neighbor.cell,
                    neighbor.site,
                    context.spacing,
                    context.seed,
                ) {
                    continue;
                }
                neighbors[index].push(neighbor_index);
                neighbors[neighbor_index].push(index);
            }
        }
    }

    neighbors
}

fn fit_component_mask(
    nodes: &[ComponentNode],
    neighbors: &[Vec<usize>],
    center_span_limit: Vec2,
) -> Result<Vec<bool>, ()> {
    let mut active = vec![true; nodes.len()];

    loop {
        let violation = total_violation(nodes, neighbors, &active, center_span_limit);
        if violation <= FIT_EPSILON {
            return Ok(active);
        }

        let components = active_components(neighbors, &active);
        let mut batches = Vec::<Vec<usize>>::new();

        for component in components {
            if component_violation(nodes, &component, center_span_limit) <= FIT_EPSILON {
                continue;
            }

            for &index in &component {
                if nodes[index].removable {
                    batches.push(vec![index]);
                }
            }

            let mut columns = HashMap::<i32, Vec<usize>>::new();
            let mut rows = HashMap::<i32, Vec<usize>>::new();
            for &index in &component {
                columns.entry(nodes[index].cell.x).or_default().push(index);
                rows.entry(nodes[index].cell.y).or_default().push(index);
            }
            batches.extend(columns.into_values().filter(|batch| {
                !batch.is_empty() && batch.iter().all(|index| nodes[*index].removable)
            }));
            batches.extend(rows.into_values().filter(|batch| {
                !batch.is_empty() && batch.iter().all(|index| nodes[*index].removable)
            }));
        }

        let mut best: Option<(Vec<usize>, f32, usize, u128)> = None;
        for batch in batches {
            let mut trial = active.clone();
            for &index in &batch {
                trial[index] = false;
            }
            let next = total_violation(nodes, neighbors, &trial, center_span_limit);
            let improvement = violation - next;
            if improvement <= FIT_EPSILON {
                continue;
            }

            let claim_sum = batch
                .iter()
                .map(|index| u128::from(nodes[*index].claim))
                .sum::<u128>();
            let replace = best.as_ref().is_none_or(|(_, best_improvement, best_len, best_claim)| {
                improvement > *best_improvement + FIT_EPSILON
                    || ((improvement - *best_improvement).abs() <= FIT_EPSILON
                        && (batch.len() < *best_len
                            || (batch.len() == *best_len && claim_sum > *best_claim)))
            });
            if replace {
                best = Some((batch, improvement, batch.len(), claim_sum));
            }
        }

        let Some((batch, _, _, _)) = best else {
            return Err(());
        };
        for index in batch {
            active[index] = false;
        }
    }
}

fn active_components(neighbors: &[Vec<usize>], active: &[bool]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; active.len()];
    let mut components = Vec::new();

    for start in 0..active.len() {
        if !active[start] || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut component = Vec::new();
        seen[start] = true;

        while let Some(index) = stack.pop() {
            component.push(index);
            for &neighbor in &neighbors[index] {
                if active[neighbor] && !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        components.push(component);
    }

    components
}

fn total_violation(
    nodes: &[ComponentNode],
    neighbors: &[Vec<usize>],
    active: &[bool],
    center_span_limit: Vec2,
) -> f32 {
    active_components(neighbors, active)
        .iter()
        .map(|component| component_violation(nodes, component, center_span_limit))
        .sum()
}

fn component_violation(
    nodes: &[ComponentNode],
    component: &[usize],
    center_span_limit: Vec2,
) -> f32 {
    if component.len() <= 1 {
        return 0.0;
    }

    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for &index in component {
        min = min.min(nodes[index].site);
        max = max.max(nodes[index].site);
    }
    let span = max - min;
    (span.x - center_span_limit.x).max(0.0) + (span.y - center_span_limit.y).max(0.0)
}

fn adjacency_claim_hash(cell: IVec2, biome_index: usize, seed: u64) -> u64 {
    region_claim_hash(cell, biome_index, seed) ^ 0x6a09_e667_f3bc_c909
}

pub(super) fn boundary_fit_interval(
    left: &BiomeFieldEntry,
    right: &BiomeFieldEntry,
    left_site: Vec2,
    right_site: Vec2,
) -> Option<BoundaryFitInterval> {
    let delta = right_site - left_site;
    let distance = delta.length();
    if distance <= FIT_EPSILON {
        return None;
    }
    let direction = delta / distance;
    let left_min = directional_radius(left.size.x.min, left.size.z.min, direction);
    let left_max = directional_radius(left.size.x.max, left.size.z.max, direction);
    let right_min = directional_radius(right.size.x.min, right.size.z.min, direction);
    let right_max = directional_radius(right.size.x.max, right.size.z.max, direction);

    boundary_interval_for_radii(left_min, left_max, right_min, right_max, distance)
}

fn boundary_interval_for_radii(
    left_min: f32,
    left_max: f32,
    right_min: f32,
    right_max: f32,
    distance: f32,
) -> Option<BoundaryFitInterval> {
    let min_from_left = left_min.max(distance - right_max).max(0.0);
    let max_from_left = left_max.min(distance - right_min).min(distance);
    (min_from_left <= max_from_left + FIT_EPSILON).then_some(BoundaryFitInterval {
        min_from_left,
        max_from_left: max_from_left.max(min_from_left),
    })
}

fn directional_radius(radius_x: f32, radius_z: f32, direction: Vec2) -> f32 {
    let radius_x = radius_x.max(FIT_EPSILON);
    let radius_z = radius_z.max(FIT_EPSILON);
    1.0 / ((direction.x / radius_x).powi(2) + (direction.y / radius_z).powi(2)).sqrt()
}

/// Fits a local weighted Voronoi (power diagram) to every different-biome
/// border. The constraints are solved as exact difference constraints, so the
/// loop count is derived from the graph size rather than an arbitrary fitting
/// pass cap. A negative cycle means the authored min/max intervals around this
/// local graph are mathematically incompatible.
pub(super) fn fit_surface_site_weights(
    sampled_sites: &[(IVec2, Vec2, f32, Option<usize>)],
    biomes: &[BiomeFieldEntry],
    spacing: Vec2,
    seed: u64,
) -> Result<Vec<f32>, String> {
    let mut constraints = Vec::<(usize, usize, f32)>::new();

    for left_index in 0..sampled_sites.len() {
        let (left_cell, left_site, _, left_biome) = sampled_sites[left_index];
        let left_biome = left_biome.expect("surface biome site must be resolved");
        for right_index in (left_index + 1)..sampled_sites.len() {
            let (right_cell, right_site, _, right_biome) = sampled_sites[right_index];
            let right_biome = right_biome.expect("surface biome site must be resolved");
            if left_biome == right_biome
                || !surface_sites_share_border(
                    left_cell,
                    left_site,
                    right_cell,
                    right_site,
                    spacing,
                    seed,
                )
            {
                continue;
            }

            let Some(interval) = boundary_fit_interval(
                &biomes[left_biome],
                &biomes[right_biome],
                left_site,
                right_site,
            ) else {
                return Err(format!(
                    "{} at {left_cell:?} and {} at {right_cell:?} have no boundary position compatible with both authored min/max ranges",
                    biomes[left_biome].id, biomes[right_biome].id,
                ));
            };

            let distance = left_site.distance(right_site);
            let distance_squared = distance * distance;
            let min_difference =
                2.0 * distance * interval.min_from_left - distance_squared;
            let max_difference =
                2.0 * distance * interval.max_from_left - distance_squared;

            // w_left - w_right <= max_difference
            constraints.push((right_index, left_index, max_difference));
            // w_right - w_left <= -min_difference
            constraints.push((left_index, right_index, -min_difference));
        }
    }

    let mut weights = vec![0.0_f32; sampled_sites.len()];
    if constraints.is_empty() {
        return Ok(weights);
    }

    for pass in 0..sampled_sites.len() {
        let mut changed = false;
        for &(from, to, maximum_delta) in &constraints {
            let maximum = weights[from] + maximum_delta;
            if weights[to] > maximum + FIT_EPSILON {
                weights[to] = maximum;
                changed = true;
            }
        }

        if !changed {
            let origin = weights[0];
            for weight in &mut weights {
                *weight -= origin;
            }
            return Ok(weights);
        }

        if pass + 1 == sampled_sites.len() {
            return Err("surface boundary fitting constraints contain a negative cycle".to_owned());
        }
    }

    unreachable!("difference-constraint fitting must converge or detect a negative cycle")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(cell_x: i32, site_x: f32, removable: bool, claim: u64) -> ComponentNode {
        ComponentNode {
            cell: IVec2::new(cell_x, 0),
            site: Vec2::new(site_x, 0.0),
            removable,
            claim,
        }
    }

    #[test]
    fn boundary_fit_moves_space_to_the_side_that_needs_its_minimum() {
        let fit = boundary_interval_for_radii(120.0, 420.0, 180.0, 180.0, 440.0)
            .expect("pair should have an exact fitted boundary");
        assert!((fit.min_from_left - 260.0).abs() <= FIT_EPSILON);
        assert!((fit.max_from_left - 260.0).abs() <= FIT_EPSILON);
    }

    #[test]
    fn boundary_fit_rejects_ranges_that_cannot_cover_the_gap() {
        assert!(boundary_interval_for_radii(80.0, 180.0, 80.0, 180.0, 440.0).is_none());
    }

    #[test]
    fn iterative_component_fit_cuts_a_long_run_until_maximum_can_hold_minimum() {
        let nodes = vec![
            node(0, 0.0, true, 1),
            node(1, 440.0, true, 2),
            node(2, 880.0, true, 3),
        ];
        let neighbors = vec![vec![1], vec![0, 2], vec![1]];
        // min=120, max=420 => site centers may span at most 600 blocks.
        let active = fit_component_mask(&nodes, &neighbors, Vec2::new(600.0, 600.0))
            .expect("the run can be fitted by yielding one site");
        assert!(active.iter().filter(|active| **active).count() <= 2);
        assert!(
            total_violation(&nodes, &neighbors, &active, Vec2::new(600.0, 600.0))
                <= FIT_EPSILON
        );
    }

    #[test]
    fn iterative_component_fit_protects_a_site_without_an_alternative() {
        let nodes = vec![node(0, 0.0, false, 1), node(1, 440.0, true, 2)];
        let neighbors = vec![vec![1], vec![0]];
        let active = fit_component_mask(&nodes, &neighbors, Vec2::new(200.0, 200.0))
            .expect("the removable side should yield");
        assert!(active[0]);
        assert!(!active[1]);
    }
}
