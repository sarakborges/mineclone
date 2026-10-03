use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::IVec3,
};

use crate::voxel::{
    coordinates::ChunkCoord, lighting::DirectLightingSeedResult, meshlet::ChunkMeshletMask,
};

/// Owns one-time presentation activation state for resident chunks. This is
/// derived runtime state: losing it may require reseeding/remeshing, but it is
/// never authoritative world data.
#[derive(Default)]
pub(super) struct InitialPresentationState {
    lighting_seeded: HashSet<ChunkCoord>,
    lighting_seed_results: HashMap<ChunkCoord, DirectLightingSeedResult>,
    lighting_activated: HashSet<ChunkCoord>,
    mesh_seed_catchup: HashMap<ChunkCoord, ChunkMeshletMask>,
}

impl InitialPresentationState {
    pub(super) fn mark_lighting_seeded(&mut self, coord: IVec3) -> bool {
        self.lighting_seeded.insert(ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn store_lighting_seed_result(
        &mut self,
        coord: IVec3,
        result: DirectLightingSeedResult,
    ) {
        self.lighting_seed_results
            .insert(ChunkCoord::from_ivec3(coord), result);
    }

    pub(super) fn take_lighting_seed_result(
        &mut self,
        coord: IVec3,
    ) -> Option<DirectLightingSeedResult> {
        self.lighting_seed_results
            .remove(&ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn mark_lighting_activated(&mut self, coord: IVec3) -> bool {
        self.lighting_activated
            .insert(ChunkCoord::from_ivec3(coord))
    }

    pub(super) fn add_mesh_seed_catchup(&mut self, coord: IVec3, meshlets: ChunkMeshletMask) {
        let coord = ChunkCoord::from_ivec3(coord);
        let combined = self
            .mesh_seed_catchup
            .get(&coord)
            .copied()
            .unwrap_or_default()
            .union(meshlets);
        self.mesh_seed_catchup.insert(coord, combined);
    }

    pub(super) fn mesh_seed_catchup(&self, coord: IVec3) -> Option<ChunkMeshletMask> {
        self.mesh_seed_catchup
            .get(&ChunkCoord::from_ivec3(coord))
            .copied()
    }

    pub(super) fn clear_mesh_seed_catchup(&mut self, coord: IVec3) {
        self.mesh_seed_catchup
            .remove(&ChunkCoord::from_ivec3(coord));
    }

    pub(super) fn forget(&mut self, coord: IVec3) {
        let coord = ChunkCoord::from_ivec3(coord);
        self.lighting_seeded.remove(&coord);
        self.lighting_seed_results.remove(&coord);
        self.lighting_activated.remove(&coord);
        self.mesh_seed_catchup.remove(&coord);
    }
}
