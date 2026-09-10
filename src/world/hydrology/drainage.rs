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

pub(super) fn drainage_node(
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

pub(super) fn drainage_neighbors(
    cell: IVec2,
    seed: u64,
    sample: &mut impl FnMut(Vec2) -> HydrologySurfaceSample,
) -> Vec<DrainageNode> {
    let mut neighbors = Vec::with_capacity(8);

    for dz in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dz == 0 {
                continue;
            }

            neighbors.push(drainage_node(cell + IVec2::new(dx, dz), seed, sample));
        }
    }

    neighbors
}

pub(super) fn select_downstream(
    source: DrainageNode,
    neighbors: &[DrainageNode],
) -> Option<DrainageNode> {
    let downstream = neighbors
        .iter()
        .copied()
        .min_by(|left, right| left.elevation.total_cmp(&right.elevation))?;

    (downstream.elevation + RIVER_MINIMUM_DROP < source.elevation).then_some(downstream)
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
