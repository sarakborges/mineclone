use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::biome_hydrology::BiomeHydrologyRules;

use super::{
    constants::{
        HYDROLOGY_REGION_SIZE, RIVER_BASIN_ESCAPE_RADIUS_CELLS, RIVER_MINIMUM_DROP,
        RIVER_OCEAN_OUTLET_RADIUS_CELLS, RIVER_ROUTE_VARIATION,
    },
    math::{cell_hash, hash_signed, hash_unit},
    types::HydrologySurfaceSample,
};

const OCEAN_OUTLET_MINIMUM_WATER_DEPTH: f32 = 0.5;

#[derive(Clone, Copy, Debug)]
pub(super) struct DrainageNode {
    pub position: Vec2,
    pub elevation: f32,
    pub ocean_weight: f32,
    pub biome_hydrology: BiomeHydrologyRules,
}

pub(super) struct DrainageNetwork<'a, F>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    seed: u64,
    sea_level: f32,
    sample: &'a mut F,
    nodes: HashMap<IVec2, DrainageNode>,
    downstream: HashMap<IVec2, Option<IVec2>>,
}

impl<'a, F> DrainageNetwork<'a, F>
where
    F: FnMut(Vec2) -> HydrologySurfaceSample,
{
    pub fn new(seed: u64, sea_level: f32, sample: &'a mut F) -> Self {
        Self {
            seed,
            sea_level,
            sample,
            nodes: HashMap::new(),
            downstream: HashMap::new(),
        }
    }

    pub fn is_wet_ocean(&self, node: DrainageNode) -> bool {
        node.ocean_weight > f32::EPSILON
            && node.elevation <= self.sea_level - OCEAN_OUTLET_MINIMUM_WATER_DEPTH
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

    pub fn surface_sample_at(&mut self, position: Vec2) -> HydrologySurfaceSample {
        (self.sample)(position)
    }

    pub fn downstream_cell(&mut self, cell: IVec2) -> Option<IVec2> {
        if let Some(cached) = self.downstream.get(&cell).copied() {
            return cached;
        }

        let source = self.node(cell);
        let downstream = self
            .ocean_outlet_cell(cell, source, 1)
            .or_else(|| {
                self.ocean_outlet_cell(cell, source, RIVER_OCEAN_OUTLET_RADIUS_CELLS)
            })
            .or_else(|| self.best_lower_cell(cell, source, 1))
            .or_else(|| self.best_lower_cell(cell, source, RIVER_BASIN_ESCAPE_RADIUS_CELLS));

        self.downstream.insert(cell, downstream);
        downstream
    }

    fn ocean_outlet_cell(
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
                if !self.is_wet_ocean(candidate)
                    || !self.river_route_is_open(source.position, candidate.position, true)
                {
                    continue;
                }

                let distance = Vec2::new(dx as f32, dz as f32).length();
                let hash = cell_hash(
                    candidate_cell,
                    cell_hash(source_cell, self.seed ^ 0x243f_6a88_85a3_08d3),
                );
                let score =
                    distance + (1.0 - candidate.ocean_weight.clamp(0.0, 1.0)) * 0.35
                        + hash_unit(hash) * 0.12;

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
        let mut candidates = Vec::new();

        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dz == 0 {
                    continue;
                }

                let candidate_cell = source_cell + IVec2::new(dx, dz);
                let candidate = self.node(candidate_cell);
                if !candidate.biome_hydrology.can_generate_river
                    || candidate.elevation + RIVER_MINIMUM_DROP >= source.elevation
                {
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
                candidates.push((candidate_cell, candidate, score));
            }
        }

        candidates.sort_unstable_by(|left, right| {
            left.2
                .total_cmp(&right.2)
                .then_with(|| compare_cell(left.0, right.0))
        });
        candidates
            .into_iter()
            .find(|(_, candidate, _)| {
                self.river_route_is_open(source.position, candidate.position, false)
            })
            .map(|(cell, _, _)| cell)
    }

    fn river_route_is_open(
        &mut self,
        from: Vec2,
        to: Vec2,
        allow_ocean_transition: bool,
    ) -> bool {
        const SAMPLE_SPACING: f32 = HYDROLOGY_REGION_SIZE * 0.5;

        let distance = from.distance(to);
        let steps = (distance / SAMPLE_SPACING).ceil().max(1.0) as usize;
        (1..steps).all(|index| {
            let t = index as f32 / steps as f32;
            let sample = (self.sample)(from.lerp(to, t));
            sample.biome_hydrology.can_generate_river
                || (allow_ocean_transition && sample.ocean_weight > f32::EPSILON)
        })
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

pub(super) fn surface_sample_is_wet_ocean(
    sample: HydrologySurfaceSample,
    sea_level: f32,
) -> bool {
    sample.ocean_weight > f32::EPSILON
        && sample.elevation <= sea_level - OCEAN_OUTLET_MINIMUM_WATER_DEPTH
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
        ocean_weight: surface.ocean_weight,
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
    fn downstream_route_does_not_jump_across_a_disabled_river_biome() {
        let mut sample = |position: Vec2| HydrologySurfaceSample {
            elevation: 120.0 - position.x * 0.02,
            ocean_weight: 0.0,
            biome_hydrology: BiomeHydrologyRules {
                can_generate_river: !(position.x > 60.0 && position.x < 190.0),
                ..Default::default()
            },
        };
        let mut network = DrainageNetwork::new(42, 90.0, &mut sample);
        let source = network.node(IVec2::ZERO);
        let blocked = network.node(IVec2::new(2, 0));

        assert!(!network.river_route_is_open(source.position, blocked.position, false));
    }

    #[test]
    fn verified_nearby_ocean_beats_a_local_descent_away_from_the_coast() {
        let ocean_cell = IVec2::new(-3, 0);
        let mut sample = |position: Vec2| {
            let cell = (position / HYDROLOGY_REGION_SIZE).floor().as_ivec2();
            HydrologySurfaceSample {
                elevation: if cell == ocean_cell {
                    80.0
                } else if cell == IVec2::X {
                    90.0
                } else if cell == IVec2::ZERO {
                    100.0
                } else {
                    110.0
                },
                ocean_weight: if cell == ocean_cell { 1.0 } else { 0.0 },
                biome_hydrology: BiomeHydrologyRules::default(),
            }
        };
        let mut network = DrainageNetwork::new(42, 90.0, &mut sample);

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
            elevation: 90.0,
            ocean_weight: 0.4,
            biome_hydrology: BiomeHydrologyRules::default(),
        };
        let sample = HydrologySurfaceSample {
            elevation: node.elevation,
            ocean_weight: node.ocean_weight,
            biome_hydrology: node.biome_hydrology,
        };

        assert!(!surface_sample_is_wet_ocean(sample, 90.0));
    }

    #[test]
    fn submerged_ocean_biome_is_a_valid_outlet() {
        let sample = HydrologySurfaceSample {
            elevation: 80.0,
            ocean_weight: 0.7,
            biome_hydrology: BiomeHydrologyRules::default(),
        };

        assert!(surface_sample_is_wet_ocean(sample, 90.0));
    }
}
