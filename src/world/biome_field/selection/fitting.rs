use bevy::prelude::*;

use super::surface_sites_share_border;
use crate::world::biome_field::BiomeFieldEntry;

const FIT_EPSILON: f32 = 0.001;

#[derive(Clone, Copy, Debug)]
pub(super) struct BoundaryFitInterval {
    pub(super) min_from_left: f32,
    pub(super) max_from_left: f32,
}

#[derive(Debug)]
pub(super) struct SurfaceFitError {
    pub(super) message: String,
    pub(super) cells: Vec<IVec2>,
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

pub(in crate::world::biome_field) fn fit_surface_site_weights(
    sampled_sites: &[(IVec2, Vec2, f32, Option<usize>)],
    biomes: &[BiomeFieldEntry],
    spacing: Vec2,
    seed: u64,
) -> Result<Vec<f32>, String> {
    fit_surface_site_weights_detailed(sampled_sites, biomes, spacing, seed)
        .map_err(|failure| failure.message)
}

/// Fits a weighted Voronoi (power diagram) and preserves the cells involved in
/// a failed constraint. The recursive identity solver uses that witness to
/// branch only where the boundary graph actually failed instead of exploring
/// every site in the sampled window.
pub(super) fn fit_surface_site_weights_detailed(
    sampled_sites: &[(IVec2, Vec2, f32, Option<usize>)],
    biomes: &[BiomeFieldEntry],
    spacing: Vec2,
    seed: u64,
) -> Result<Vec<f32>, SurfaceFitError> {
    let mut constraints = Vec::<(usize, usize, f32)>::new();

    for left_index in 0..sampled_sites.len() {
        let (left_cell, left_site, _, left_biome) = sampled_sites[left_index];
        let left_biome = left_biome.expect("surface biome site must be resolved");
        for (right_index, &(right_cell, right_site, _, right_biome)) in sampled_sites
            .iter()
            .enumerate()
            .skip(left_index + 1)
        {
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
                return Err(SurfaceFitError {
                    message: format!(
                        "{} at {left_cell:?} and {} at {right_cell:?} have no boundary position compatible with both authored min/max ranges",
                        biomes[left_biome].id, biomes[right_biome].id,
                    ),
                    cells: vec![left_cell, right_cell],
                });
            };

            let distance = left_site.distance(right_site);
            let distance_squared = distance * distance;
            let min_difference =
                2.0 * distance * interval.min_from_left - distance_squared;
            let max_difference =
                2.0 * distance * interval.max_from_left - distance_squared;

            constraints.push((right_index, left_index, max_difference));
            constraints.push((left_index, right_index, -min_difference));
        }
    }

    let mut weights = vec![0.0_f32; sampled_sites.len()];
    if constraints.is_empty() {
        return Ok(weights);
    }

    for pass in 0..sampled_sites.len() {
        let mut changed = false;
        let mut changed_indices = Vec::new();
        for &(from, to, maximum_delta) in &constraints {
            let maximum = weights[from] + maximum_delta;
            if weights[to] > maximum + FIT_EPSILON {
                weights[to] = maximum;
                changed = true;
                changed_indices.push(from);
                changed_indices.push(to);
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
            changed_indices.sort_unstable();
            changed_indices.dedup();
            let cells = changed_indices
                .into_iter()
                .map(|index| sampled_sites[index].0)
                .collect::<Vec<_>>();
            return Err(SurfaceFitError {
                message: "surface boundary fitting constraints contain a negative cycle".to_owned(),
                cells,
            });
        }
    }

    unreachable!("difference-constraint fitting must converge or detect a negative cycle")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
