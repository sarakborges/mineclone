mod generation;
mod presentation;
mod residency;
mod selection;

use std::cmp::Reverse;

use bevy::{
    ecs::system::SystemParam,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::chunk_coord_from_position, deduplicated_queue::DeduplicatedQueue,
        lighting::PendingLightingUpdates, world::VoxelWorld,
    },
};

pub(super) use self::generation::ChunkMaterializationTasks;
use self::{
    generation::{
        MaterializationDefinitions, MaterializationRuntime, collect_materialized_chunks,
        dispatch_materialization_tasks,
    },
    presentation::publish_pending_chunk_presentations,
    residency::ChunkResidencyState,
    selection::desired_chunk_coords,
};
use super::{
    chunk_remesh::process_chunk_remesh_queue,
    chunk_rendering::ChunkRenderPool,
    chunk_system_params::VoxelContent,
    chunk_unloading::evict_distant_chunks,
    chunk_visibility::ChunkPresentationSelection,
    fluid_updates::PendingFluidUpdates,
    generator::WorldGenerator,
    lighting_updates::process_dynamic_lighting,
    render_distance::RenderDistanceSettings,
    tick::WorldTickClock,
    warp::{PendingWarp, resolve_pending_warp},
    work_budget::{WorldFrameWorkBudget, begin_world_frame_work_budget},
};

pub(super) struct ChunkStreamingPlugin;

impl Plugin for ChunkStreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMaterializationTasks>()
            .add_systems(
                OnEnter(GameState::Loading),
                reset_resource::<ChunkMaterializationTasks>,
            )
            .add_systems(
                OnEnter(GameState::Gameplay),
                reset_resource::<ChunkMaterializationTasks>,
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                reset_resource::<ChunkMaterializationTasks>,
            )
            .add_systems(
                Update,
                stream_chunks
                    .after(begin_world_frame_work_budget)
                    .before(resolve_pending_warp)
                    .before(evict_distant_chunks)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                PostUpdate,
                publish_pending_chunk_presentations
                    .after(process_dynamic_lighting)
                    .before(process_chunk_remesh_queue)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

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
    presentation_pending: DeduplicatedQueue<IVec3>,
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
        self.pending.len() > 0
            || !self.materializing.is_empty()
            || self.presentation_pending.len() > 0
    }

    pub(super) fn mesh_pressure_evicted_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.pressure_evicted_meshes.keys().copied()
    }

    pub(super) fn mesh_pressure_evicted_bytes(&self, coord: IVec3) -> Option<usize> {
        self.pressure_evicted_meshes.get(&coord).copied()
    }

    pub(super) fn recover_mesh_after_pressure(&mut self, coord: IVec3) -> bool {
        let recovered = self.pressure_evicted_meshes.remove(&coord).is_some();
        if recovered {
            self.enqueue_presentation(coord);
        }
        recovered
    }

    pub(super) fn suppress_mesh_for_pressure(&mut self, coord: IVec3, bytes: usize) {
        self.presentation_pending.remove(coord);
        self.pressure_evicted_meshes.insert(coord, bytes);
    }

    pub(super) fn forget_initial_lighting_seeded(&mut self, coord: IVec3) {
        self.presentation_pending.remove(coord);
        self.pressure_evicted_meshes.remove(&coord);
    }

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
            let priority = chunk_load_priority(*coord, center, IVec2::ZERO);
            (Reverse(priority.1), coord.y, coord.z, coord.x)
        });
        for coord in retired {
            self.residency.enqueue_retired(coord);
        }

        let retained_presentations = self
            .presentation_pending
            .values()
            .filter(|coord| desired.contains(coord))
            .collect::<Vec<_>>();
        self.presentation_pending.clear();
        for coord in retained_presentations {
            self.presentation_pending.enqueue(coord);
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

    pub(super) fn enqueue_presentation(&mut self, coord: IVec3) {
        if coord.y >= 0
            && self.keeps_loaded(coord)
            && !self.pressure_evicted_meshes.contains_key(&coord)
        {
            self.presentation_pending.enqueue(coord);
        }
    }

    pub(super) fn pop_presentation_by_priority(
        &mut self,
        selection: &ChunkPresentationSelection,
    ) -> Option<IVec3> {
        let center = self.center?;
        let movement_direction = self.movement_direction;
        let selected = self
            .presentation_pending
            .values()
            .filter(|coord| selection.retains_render_mesh(*coord))
            .min_by_key(|coord| chunk_load_priority(*coord, center, movement_direction))?;
        let removed = self.presentation_pending.remove(selected);
        debug_assert!(removed, "selected presentation chunk must remain pending");
        Some(selected)
    }

    pub(super) fn is_materializing(&self, coord: IVec3) -> bool {
        self.materializing.contains(&coord)
    }

    pub(super) fn mark_materializing(&mut self, coord: IVec3) {
        let inserted = self.materializing.insert(coord);
        debug_assert!(
            inserted,
            "chunk cannot materialize twice concurrently: {coord:?}"
        );
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

#[derive(SystemParam)]
struct ChunkStreamingInputs<'w, 's> {
    generator: Res<'w, WorldGenerator>,
    content: VoxelContent<'w>,
    render_pool: Res<'w, ChunkRenderPool>,
    world_ticks: Res<'w, WorldTickClock>,
    render_distance: Res<'w, RenderDistanceSettings>,
    pending_warp: Res<'w, PendingWarp>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    frame_budget: Res<'w, WorldFrameWorkBudget>,
}

#[derive(SystemParam)]
struct ChunkStreamingRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    state: ResMut<'w, ChunkStreamingState>,
    tasks: ResMut<'w, ChunkMaterializationTasks>,
    pending_fluid: ResMut<'w, PendingFluidUpdates>,
    pending_lighting: ResMut<'w, PendingLightingUpdates>,
    presentation_selection: ResMut<'w, ChunkPresentationSelection>,
}

fn stream_chunks(inputs: ChunkStreamingInputs, mut runtime: ChunkStreamingRuntime) {
    if inputs.generator.is_changed() {
        runtime.tasks.restart_for_generator_change();
        runtime.state.restart_materializations();
    }

    let feet_position = inputs.player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = inputs
        .pending_warp
        .streaming_center()
        .unwrap_or_else(|| chunk_coord_from_position(feet_position));
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = inputs
        .pending_warp
        .streaming_horizontal_radius()
        .unwrap_or_else(|| inputs.render_distance.chunks());
    runtime
        .presentation_selection
        .sync_from_streaming(Some(center), horizontal_radius);

    if runtime.state.selection_needs_rebuild(center, horizontal_radius) {
        let desired = desired_chunk_coords(&inputs.generator, center, horizontal_radius);
        runtime
            .state
            .rebuild_selection(center, horizontal_radius, desired);

        let mut missing = runtime
            .state
            .residency
            .desired
            .iter()
            .copied()
            .filter(|coord| {
                runtime.world.chunk(*coord).is_none() && !runtime.state.is_materializing(*coord)
            })
            .collect::<Vec<_>>();
        missing.sort_unstable_by_key(|coord| {
            chunk_load_priority(*coord, center, runtime.state.movement_direction())
        });
        for coord in missing {
            runtime.state.enqueue_pending(coord);
        }

        let resident_unpresented = runtime
            .state
            .residency
            .desired
            .iter()
            .copied()
            .filter(|coord| {
                runtime.world.chunk(*coord).is_some() && !inputs.render_pool.contains(*coord)
            })
            .collect::<Vec<_>>();
        for coord in resident_unpresented {
            runtime.state.enqueue_presentation(coord);
        }
    }

    let current_tick = inputs.world_ticks.current_tick();
    let definitions = MaterializationDefinitions {
        blocks: &inputs.content.blocks,
        fluids: &inputs.content.fluids,
        secondary_properties: &inputs.content.secondary_properties,
    };
    let mut materialization_runtime = MaterializationRuntime::new(
        &mut runtime.world,
        &mut runtime.pending_fluid,
        &mut runtime.pending_lighting,
        definitions,
        current_tick,
        inputs.frame_budget.deadline(),
    );
    collect_materialized_chunks(
        &mut runtime.tasks,
        &mut runtime.state,
        &mut materialization_runtime,
    );
    dispatch_materialization_tasks(
        &inputs.generator,
        &mut runtime.tasks,
        &mut runtime.state,
        &mut materialization_runtime,
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
