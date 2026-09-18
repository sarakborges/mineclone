use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::biome_hydrology::BiomeHydrology;

use super::{
    constants::{
        HYDROLOGY_REGION_SIZE, OCEAN_EXTRA_DEPTH, OCEAN_MINIMUM_DEPTH,
        RIVER_BASIN_ESCAPE_RADIUS_CELLS, RIVER_MINIMUM_DROP,
        RIVER_OCEAN_OUTLET_RADIUS_CELLS, RIVER_ROUTE_VARIATION,
    },
    math::{cell_hash, hash_signed, hash_unit, lerp, ocean_strength},
    types::HydrologySurfaceSample,
};

const OCEAN_OUTLET_MINIMUM_WATER_DEPTH: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
pub(super) struct DrainageNode {
    pub position: Vec2,
    pub elevation: f32,
    pub continentalness: f32,
    pub biome_hydrology: BiomeHydrology,
}

pub(super) struct DrainageNetwork<'a, F>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    seed: u64,
    ocean_threshold: f32,
    sea_level: f32,
    ocean_weight: f32,
    sample: &'a mut F,
    nodes: HashMap<IVec2, DrainageNode>,
    downstream: HashMap<IVec2, Option<IVec2>>,
}

impl<'a, F> DrainageNetwork<'a, F>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    pub fn new(
        seed: u64,
        ocean_threshold: f32,
        sea_level: f32,
        ocean_weight: f32,
        sample: &'a mut F,
    ) -> Self {
        Self {
            seed,
            ocean_threshold,
            sea_level,
            ocean_weight,
            sample,
            nodes: HashMap::new(),
            downstream: HashMap::new(),
        }
    }

    pub fn ocean_threshold(&self) -> f32 {
        self.ocean_threshold
    }

    pub fn is_wet_ocean(&self, node: DrainageNode) -> bool {
        wet_ocean_floor(node, self.sea_level, self.ocean_weight)
            .is_some_and(|floor| floor <= self.sea_level - OCEAN_OUTLET_MINIMUM_WATER_DEPTH)
    }

    pub fn node(&mut self, cell: IVec2) -> DrainageNode {
        if let Some(node) = self.nodes.get(&cell).copied() {
            return node;
        }

        let node = drainage_node(cell, self.seed, self.sample);
        self.nodes.insert(cell, node);
        node
    }

    pub fn neighbor_nodes(&mut self, cell: IVec2) -> Vec<DrainageNode> {
        self.neighbors(cell)
            .into_iter()
            .map(|(_, node)| node)
            .collect()
    }

    pub fn surface_elevation_at(&mut self, position: Vec2) -> f32 {
        (self.sample)(position).elevation
    }

    pub fn surface_sample_at(&mut self, position: Vec2) -> HydrologySurfaceSample {
        (self.sample)(position)
    }

    pub fn downstream_cell(&mut self, cell: IVec2) -> Option<IVec2> {
        if let Some(cached) = self.downstream.get(&cell).copied() {
            return cached;
        }

        let source = self.node(cell);
        // A nearby *wet* ocean is a verified outlet. Prefer it before taking a
        // locally lower step that may lead away from the coast into a closed
        // depression. Dry ocean-transition fringes remain ineligible because
        // ocean_outlet_cell checks the interpolated physical ocean floor.
        let downstream = self
            .ocean_outlet_cell(cell, 1)
            .or_else(|| self.ocean_outlet_cell(cell, RIVER_OCEAN_OUTLET_RADIUS_CELLS))
            .or_else(|| self.best_lower_cell(cell, source, 1))
            .or_else(|| self.best_lower_cell(cell, source, RIVER_BASIN_ESCAPE_RADIUS_CELLS));

        self.downstream.insert(cell, downstream);
        downstream
    }

    fn ocean_outlet_cell(&mut self, source_cell: IVec2, radius: i32) -> Option<IVec2> {
        let mut best: Option<(IVec2, f32)> = None;

        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dz == 0 {
                    continue;
                }

                let candidate_cell = source_cell + IVec2::new(dx, dz);
                let candidate = self.node(candidate_cell);
                // A low continentalness value alone does not imply actual water:
                // at the outer ocean blend the interpolated floor may still be
                // above sea level. Never terminate a river on this dry fringe.
                if !self.is_wet_ocean(candidate) {
                    continue;
                }

                let distance = Vec2::new(dx as f32, dz as f32).length();
                let hash = cell_hash(
                    candidate_cell,
                    cell_hash(source_cell, self.seed ^ 0x243f_6a88_85a3_08d3),
                );
                let score = distance + candidate.continentalness * 0.35 + hash_unit(hash) * 0.12;

                if best.as_ref().is_none_or(|(best_cell, best_score)| {
                    score.total_cmp(best_score).is_lt()
                        || (score.total_cmp(best_score).is_eq()
                            && compare_cell(candidate_cell, *best_cell).is_lt())
                }) {
                    best = Some((candidate_cell, score));
                }
            }
        }

        best.map(|(cell, _)| cell)
    }

    fn best_lower_cell(
        &mut self,
        source_cell: IVec2,
        source: DrainageNode,
        radius: i32,
    ) -> Option<IVec2> {
        let mut best: Option<(IVec2, f32)> = None;

        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dz == 0 {
                    continue;
                }

                let candidate_cell = source_cell + IVec2::new(dx, dz);
                let candidate = self.node(candidate_cell);
                if candidate.elevation + RIVER_MINIMUM_DROP >= source.elevation {
                    continue;
                }

                let distance = Vec2::new(dx as f32, dz as f32).length();
                let score = downstream_score(
                    source_cell,
                    candidate_cell,
                    candidate.elevation,
                    distance,
                    self.seed,
                );

                if best.as_ref().is_none_or(|(best_cell, best_score)| {
                    score.total_cmp(best_score).is_lt()
                        || (score.total_cmp(best_score).is_eq()
                            && compare_cell(candidate_cell, *best_cell).is_lt())
                }) {
                    best = Some((candidate_cell, score));
                }
            }
        }

        best.map(|(cell, _)| cell)
    }

    fn neighbors(&mut self, cell: IVec2) -> Vec<(IVec2, DrainageNode)> {
        let mut neighbors = Vec::with_capacity(8);

        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }

                let neighbor_cell = cell + IVec2::new(dx, dz);
                neighbors.push((neighbor_cell, self.node(neighbor_cell)));
            }
        }

        neighbors
    }
}

fn wet_ocean_floor(node: DrainageNode, sea_level: f32, ocean_weight: f32) -> Option<f32> {
    let strength = ocean_strength(node.continentalness, ocean_weight);
    if strength <= 0.0 {
        return None;
    }

    let target_floor = sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
    Some(lerp(node.elevation, target_floor, strength))
}

fn downstream_score(
    source_cell: IVec2,
    candidate_cell: IVec2,
    elevation: f32,
    distance: f32,
    seed: u64,
) -> f32 {
    let source_hash = cell_hash(source_cell, seed ^ 0xa409_3822_299f_31d0);
    let route_hash = cell_hash(candidate_cell, source_hash);
    let route_variation = hash_unit(route_hash) * RIVER_ROUTE_VARIATION;
    let distance_penalty = (distance - 1.0).max(0.0) * 0.2;

    elevation + route_variation + distance_penalty
}

fn compare_cell(left: IVec2, right: IVec2) -> std::cmp::Ordering {
    left.x.cmp(&right.x).then_with(|| left.y.cmp(&right.y))
}

fn drainage_node(
    cell: IVec2,
    seed: u64,
    sample: &mut impl FnMut(Vec2) -> HydrologySurfaceSample,
) -> DrainageNode {
    let position = drainage_position(cell, seed);
    let surface = sample(position);

    DrainageNode {
        position,
        elevation: surface.elevation,
        continentalness: surface.continentalness,
        biome_hydrology: surface.biome_hydrology,
    }
}

pub(super) fn drainage_position(cell: IVec2, seed: u64) -> Vec2 {
    let base = (cell.as_vec2() + Vec2::splat(0.5)) * HYDROLOGY_REGION_SIZE;
    let hash = cell_hash(cell, seed);
    let jitter = Vec2::new(
        hash_signed(hash) * HYDROLOGY_REGION_SIZE * 0.36,
        hash_signed(hash.rotate_left(31)) * HYDROLOGY_REGION_SIZE * 0.36,
    );

    base + jitter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_variation_is_deterministic() {
        let source = IVec2::new(2, -4);

        assert_eq!(
            downstream_score(source, IVec2::new(3, -3), 70.0, 1.0, 42),
            downstream_score(source, IVec2::new(3, -3), 70.0, 1.0, 42),
        );
    }

    #[test]
    fn lower_elevation_still_dominates_route_variation() {
        let source = IVec2::ZERO;
        let high = downstream_score(source, IVec2::X, 70.0, 1.0, 42);
        let low = downstream_score(source, IVec2::Y, 65.0, 1.0, 42);

        assert!(low < high);
    }

    #[test]
    fn verified_nearby_ocean_beats_a_local_descent_away_from_the_coast() {
        let ocean_cell = IVec2::new(-3, 0);
        let mut sample = |position: Vec2| {
            let cell = (position / HYDROLOGY_REGION_SIZE).floor().as_ivec2();
            HydrologySurfaceSample {
                elevation: if cell == IVec2::X {
                    90.0
                } else if cell == IVec2::ZERO {
                    100.0
                } else {
                    110.0
                },
                continentalness: if cell == ocean_cell { 0.0 } else { 0.8 },
                biome_hydrology: BiomeHydrology::default(),
            }
        };
        let mut network = DrainageNetwork::new(42, 0.45, 90.0, 1.0, &mut sample);

        let inland_descent = network.node(IVec2::X);
        let ocean = network.node(ocean_cell);
        assert!(!network.is_wet_ocean(inland_descent));
        assert!(network.is_wet_ocean(ocean));
        assert_eq!(network.downstream_cell(IVec2::ZERO), Some(ocean_cell));
    }

    #[test]
    fn dry_ocean_fringe_is_not_a_valid_river_destination() {
        let node = DrainageNode {
            position: Vec2::ZERO,
            elevation: 100.0,
            continentalness: 0.44,
            biome_hydrology: BiomeHydrology::default(),
        };
        assert!(wet_ocean_floor(node, 90.0, 1.0).unwrap() > 90.0);
    }

    #[test]
    fn deep_ocean_has_a_submerged_outlet() {
        let node = DrainageNode {
            position: Vec2::ZERO,
            elevation: 100.0,
            continentalness: 0.10,
            biome_hydrology: BiomeHydrology::default(),
        };
        assert!(wet_ocean_floor(node, 90.0, 1.0).unwrap() <= 86.0);
    }
}
