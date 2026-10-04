mod residency;

use bevy::{platform::collections::HashMap, prelude::*};

use self::residency::ChunkResidencyState;

pub(super) type ChunkLoadPriority = (i64, i64, i32, i32, i32, i32);

/// Streaming remains the runtime owner of residency while generation and
/// selection are rebuilt. Phase 8 will add the new interest-selection policy.
#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    residency: ChunkResidencyState,
    pressure_evicted_meshes: HashMap<IVec3, usize>,
}

impl ChunkStreamingState {
    pub(super) fn center(&self) -> Option<IVec3> {
        None
    }

    pub(super) fn movement_direction(&self) -> IVec2 {
        IVec2::ZERO
    }

    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.residency.keeps_loaded(coord)
    }

    pub(super) fn enqueue_retired(&mut self, coord: IVec3) {
        self.residency.enqueue_retired(coord);
    }

    pub(super) fn pop_retired_outside_horizontal_radius(
        &mut self,
        center: IVec3,
        horizontal_radius: i32,
    ) -> Option<IVec3> {
        self.residency
            .pop_retired_outside_horizontal_radius(center, horizontal_radius)
    }

    pub(in crate::world) fn generated_chunk_is_unpublished(&self, _coord: IVec3) -> bool {
        false
    }

    pub(in crate::world) fn generated_fluid_settling_owns_mutation(&self, _coord: IVec3) -> bool {
        false
    }

    pub(super) fn has_renderable_streaming_backlog(&self) -> bool {
        false
    }

    pub(super) fn mesh_pressure_evicted_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.pressure_evicted_meshes.keys().copied()
    }

    pub(super) fn mesh_pressure_evicted_bytes(&self, coord: IVec3) -> Option<usize> {
        self.pressure_evicted_meshes.get(&coord).copied()
    }

    pub(super) fn recover_mesh_after_pressure(&mut self, coord: IVec3) -> bool {
        self.pressure_evicted_meshes.remove(&coord).is_some()
    }

    pub(super) fn suppress_mesh_for_pressure(&mut self, coord: IVec3, bytes: usize) {
        self.pressure_evicted_meshes.insert(coord, bytes);
    }

    pub(super) fn forget_initial_lighting_seeded(&mut self, _coord: IVec3) {}
}

pub(super) fn chunk_load_priority(
    coord: IVec3,
    center: IVec3,
    movement_direction: IVec2,
) -> ChunkLoadPriority {
    let dx = i64::from(coord.x) - i64::from(center.x);
    let dy = i64::from(coord.y) - i64::from(center.y);
    let dz = i64::from(coord.z) - i64::from(center.z);
    let horizontal_distance = dx * dx + dz * dz;
    let total_distance = horizontal_distance + dy * dy;
    let forward = dx * i64::from(movement_direction.x) + dz * i64::from(movement_direction.y);
    let directional_band = if movement_direction == IVec2::ZERO || forward == 0 {
        1
    } else if forward > 0 {
        0
    } else {
        2
    };

    (
        horizontal_distance,
        total_distance,
        directional_band,
        coord.y,
        coord.z,
        coord.x,
    )
}
