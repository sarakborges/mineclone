use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::biome_hydrology::BiomeHydrology;

use super::{
    constants::{
        HYDROLOGY_REGION_SIZE, RIVER_BASIN_ESCAPE_RADIUS_CELLS, RIVER_MINIMUM_DROP,
        RIVER_ROUTE_VARIATION,
    },
    math::{cell_hash, hash_signed, hash_unit},
    types::HydrologySurfaceSample,
};

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
    sample: &'a mut F,
    nodes: HashMap<IVec2, DrainageNode>,
    downstream: HashMap<IVec2, Option<IVec2>>,
}

impl<'a, F> DrainageNetwork<'a, F>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    pub fn new(seed: u64, sample: &'a mut F) -> Self {
        Self {
            seed,
            sample,
            nodes: HashMap::new(),
            downstream: HashMap::new(),
        }
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

    pub fn downstream_cell(&mut self, cell: IVec2) -> Option<IVec2> {
        if let Some(cached) = self.downstream.get(&cell).copied() {
            return cached;
        }

        let source = self.node(cell);
        let downstream = self
            .best_lower_cell(cell, source, 1)
            .or_else(|| self.best_lower_cell(cell, source, RIVER_BASIN_ESCAPE_RADIUS_CELLS));

        self.downstream.insert(cell, downstream);
        downstream
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

                if best
                    .as_ref()
                    .is_none_or(|(best_cell, best_score)| {
                        score.total_cmp(best_score).is_lt()
                            || (score.total_cmp(best_score).is_eq()
                                && compare_cell(candidate_cell, *best_cell).is_lt())
                    })
                {
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
        hash_signed(hash) * HYDROLOGY_REGION_SIZE * 0.22,
        hash_signed(hash.rotate_left(31)) * HYDROLOGY_REGION_SIZE * 0.22,
    );

    base + jitter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_variation_is_deterministic() {
        let source = IVec2::new(2, -4);
        let candidate = IVec2::new(3, -3);

        assert_eq!(
            downstream_score(source, candidate, 70.0, 1.0, 42),
            downstream_score(source, candidate, 70.0, 1.0, 42)
        );
    }

    #[test]
    fn lower_elevation_still_dominates_small_route_variation() {
        let source = IVec2::ZERO;
        let high = downstream_score(source, IVec2::X, 70.0, 1.0, 42);
        let low = downstream_score(source, IVec2::Y, 65.0, 1.0, 42);

        assert!(low < high);
    }
}
