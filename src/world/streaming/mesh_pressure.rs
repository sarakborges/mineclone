use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::IVec3,
};

use crate::voxel::coordinates::ChunkCoord;

/// Tracks render meshes evicted only because the mesh-memory high watermark was
/// exceeded. Logical residency is unaffected; entries are eligible for
/// presentation recovery when pressure drops.
#[derive(Default)]
pub(super) struct MeshPressureState {
    evicted: HashMap<ChunkCoord, usize>,
}

impl MeshPressureState {
    pub(super) fn suppress(&mut self, coord: IVec3, bytes: usize) {
        self.evicted.insert(ChunkCoord::from_ivec3(coord), bytes);
    }

    pub(super) fn recover(&mut self, coord: IVec3) -> bool {
        self.evicted
            .remove(&ChunkCoord::from_ivec3(coord))
            .is_some()
    }

    pub(super) fn coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.evicted.keys().copied().map(ChunkCoord::as_ivec3)
    }

    pub(super) fn bytes(&self, coord: IVec3) -> Option<usize> {
        self.evicted
            .get(&ChunkCoord::from_ivec3(coord))
            .copied()
    }

    pub(super) fn contains(&self, coord: IVec3) -> bool {
        self.evicted.contains_key(&ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn len(&self) -> usize {
        self.evicted.len()
    }

    pub(super) fn retain_for_selection(
        &mut self,
        desired: &HashSet<IVec3>,
        mut keep: impl FnMut(IVec3) -> bool,
    ) {
        self.evicted.retain(|coord, _| {
            let coord = coord.as_ivec3();
            desired.contains(&coord) && keep(coord)
        });
    }
}
