use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::biome_hydrology::BiomeHydrology;

use super::{
    constants::{HYDROLOGY_REGION_SIZE, RIVER_MINIMUM_DROP},
    math::{cell_hash, hash_signed},
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
            .neighbors(cell)
            .into_iter()
            .filter(|(_, node)| node.elevation + RIVER_MINIMUM_DROP < source.elevation)
            .min_by(|(left_cell, left), (right_cell, right)| {
                left.elevation
                    .total_cmp(&right.elevation)
                    .then_with(|| left_cell.x.cmp(&right_cell.x))
                    .then_with(|| left_cell.y.cmp(&right_cell.y))
            })
            .map(|(cell, _)| cell);

        self.downstream.insert(cell, downstream);
        downstream
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
