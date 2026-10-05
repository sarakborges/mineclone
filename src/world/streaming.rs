mod generation;
mod residency;
mod selection;

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    content::fluid::FluidRegistry,
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::chunk_coord_from_position, deduplicated_queue::DeduplicatedQueue,
        world::VoxelWorld,
    },
};

pub(super) use self::generation::ChunkMaterializationTasks;
use self::{
    generation::{collect_materialized_chunks, dispatch_materialization_tasks},
    residency::ChunkResidencyState,
    selection::desired_chunk_coords,
};
use super::{
    fluid_updates::PendingFluidUpdates, generator::WorldGenerator,
    render_distance::RenderDistanceSettings, tick::WorldTickClock, warp::PendingWarp,
    work_budget::WorldFrameWorkBudget,
};

pub(super) type ChunkLoadPriority = (i64, i64, i32, i32, i32, i32);

/// Runtime owner for generated chunk interest, materialization scheduling, and residency.
///
/// Semantic generation stays inside `WorldGenerator`; this state only decides which
/// already-deterministic chunk requests should be resident and in what runtime order
/// they should be materialized or retired.
#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    center: Option<IVec3>,
    horizontal_radius: i32,
    movement_direction: IVec2,
    residency: ChunkResidencyState,
    pending: DeduplicatedQueue<IVec3>,
    materializing: HashSet<IVec3>,
    pressure_evicted_meshes: HashMap<IVec3, usize>,
}

impl ChunkStreamingState {
    pub(super) fn center(&self) -> Option<IVec3> {
        self.center
    }

    pub(super) fn movement_direction(&self) -> IVec2 {
        self.movement_direction
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

    pub(in crate::world) fn generated_chunk_is_unpublished(&self, coord: IVec3) -> bool {
        self.materializing.contains(&coord)
    }

    pub(in crate::world) fn generated_fluid_settling_owns_mutation(&self, _coord: IVec3) -> bool {
        false
    }

    pub(super) fn has_renderable_streaming_backlog(&self) -> bool {
        !self.pending.values().next().is_none() || !self.materializing.is_empty()
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

    fn selection_needs_rebuild(&self, center: IVec3, horizontal_radius: i32) -> bool {
        self.center != Some(center) || self.horizontal_radius != horizontal_radius
    }

    fn rebuild_selection(
        &mut self,
        center: IVec3,
        horizontal_radius: i32,
        desired: HashSet<IVec3>,
    ) {
        self.update_movement_direction(center);

        let mut retired = self
            .residency
            .desired
            .iter()
            .chain(self.residency.retained.iter())
            .copied()
            .filter(|coord| !desired.contains(coord))
            .collect::<Vec<_>>();
        retired.sort_unstable_by_key(|coord| {
            let delta = *coord - center;
            (Reverse(delta.length_squared()), coord.y, coord.z, coord.x)
        });
        for coord in retired {
            self.residency.enqueue_retired(coord);
        }

        self.residency.desired = desired;
        self.residency.retained.clear();
        self.pending.clear();
        self.center = Some(center);
        self.horizontal_radius = horizontal_radius;
    }

    fn update_movement_direction(&mut self, center: IVec3) {
        let Some(previous) = self.center else {
            self.movement_direction = IVec2::ZERO;
            return;
        };
        let delta = center.xz() - previous.xz();
        if delta != IVec2::ZERO {
            self.movement_direction = IVec2::new(delta.x.signum(), delta.y.signum());
        }
    }

    pub(super) fn enqueue_pending(&mut self, coord: IVec3) {
        if coord.y >= 0 && self.keeps_loaded(coord) && !self.materializing.contains(&coord) {
            self.pending.enqueue(coord);
        }
    }

    pub(super) fn pop_pending_by_priority(&mut self) -> Option<IVec3> {
        let center = self.center?;
        let movement_direction = self.movement_direction;
        let selected = self
            .pending
            .values()
            .min_by_key(|coord| chunk_load_priority(*coord, center, movement_direction))?;
        let removed = self.pending.remove(selected);
        debug_assert!(removed, "selected streaming chunk must remain pending");
        Some(selected)
    }

    pub(super) fn is_materializing(&self, coord: IVec3) -> bool {
        self.materializing.contains(&coord)
    }

    pub(super) fn mark_materializing(&mut self, coord: IVec3) {
        let inserted = self.materializing.insert(coord);
        debug_assert!(inserted, "chunk cannot materialize twice concurrently: {coord:?}");
    }

    pub(super) fn finish_materializing(&mut self, coord: IVec3) {
        self.materializing.remove(&coord);
    }

    fn restart_materializations(&mut self) {
        let mut interrupted = self.materializing.drain().collect::<Vec<_>>();
        interrupted.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        for coord in interrupted {
            if self.keeps_loaded(coord) {
                self.pending.enqueue(coord);
            }
        }
    }
}

pub(super) fn stream_chunks(
    generator: Res<WorldGenerator>,
    fluids: Res<FluidRegistry>,
    world_ticks: Res<WorldTickClock>,
    render_distance: Res<RenderDistanceSettings>,
    pending_warp: Res<PendingWarp>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut world: ResMut<VoxelWorld>,
    mut state: ResMut<ChunkStreamingState>,
    mut tasks: ResMut<ChunkMaterializationTasks>,
    mut pending_fluid: ResMut<PendingFluidUpdates>,
    frame_budget: Res<WorldFrameWorkBudget>,
) {
    if generator.is_changed() {
        tasks.restart_for_generator_change();
        state.restart_materializations();
    }

    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = pending_warp
        .streaming_center()
        .unwrap_or_else(|| chunk_coord_from_position(feet_position));
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = render_distance.chunks();

    if state.selection_needs_rebuild(center, horizontal_radius) {
        let desired = desired_chunk_coords(&generator, center, horizontal_radius);
        state.rebuild_selection(center, horizontal_radius, desired);

        let mut missing = state
            .residency
            .desired
            .iter()
            .copied()
            .filter(|coord| world.chunk(*coord).is_none() && !state.is_materializing(*coord))
            .collect::<Vec<_>>();
        missing.sort_unstable_by_key(|coord| {
            chunk_load_priority(*coord, center, state.movement_direction())
        });
        for coord in missing {
            state.enqueue_pending(coord);
        }
    }

    let current_tick = world_ticks.current_tick();
    collect_materialized_chunks(
        &mut tasks,
        &mut state,
        &mut world,
        &mut pending_fluid,
        &fluids,
        current_tick,
        &frame_budget,
    );
    dispatch_materialization_tasks(
        &generator,
        &mut tasks,
        &mut state,
        &mut world,
        &mut pending_fluid,
        &fluids,
        current_tick,
        &frame_budget,
    );
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
