mod residency;
mod selection_state;

use bevy::{platform::collections::HashMap, prelude::*};

use self::{residency::ChunkResidencyState, selection_state::StreamingSelectionState};

/// Streaming remains the runtime owner of interest/residency while generation
/// is rebuilt. Phase 2+ will reconnect materialization to this boundary.
#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    selection_state: StreamingSelectionState,
    residency: ChunkResidencyState,
    pressure_evicted_meshes: HashMap<IVec3, usize>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct StreamingPriorityScanDiagnostic {
    pub(super) count: usize,
    pub(super) average_micros: u64,
    pub(super) max_micros: u64,
    pub(super) max_queue_len: usize,
}

impl ChunkStreamingState {
    pub(super) fn center(&self) -> Option<IVec3> {
        self.selection_state.center()
    }

    pub(super) fn movement_direction(&self) -> IVec2 {
        self.selection_state.movement_direction()
    }

    pub(super) fn selection_revision(&self) -> u64 {
        self.residency.revision()
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

    pub(super) fn diagnostic_counts(&self) -> (usize, usize, usize, usize, usize, usize) {
        (0, 0, 0, 0, 0, self.pressure_evicted_meshes.len())
    }

    pub(super) fn diagnostic_renderable_backlog_counts(&self) -> (usize, usize, usize) {
        (0, 0, 0)
    }

    pub(super) fn diagnostic_generation_prefetch_count(&self) -> usize {
        0
    }

    pub(super) fn diagnostic_fluid_settling_counts(
        &self,
    ) -> (bool, usize, usize, usize, usize, usize, usize) {
        (false, 0, 0, 0, 0, 0, 0)
    }

    pub(super) fn take_priority_scan_diagnostics(
        &self,
    ) -> (StreamingPriorityScanDiagnostic, StreamingPriorityScanDiagnostic) {
        (
            StreamingPriorityScanDiagnostic::default(),
            StreamingPriorityScanDiagnostic::default(),
        )
    }
}
