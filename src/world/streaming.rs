mod generation;
mod meshing;
mod selection;
mod surface_cache;

use bevy::{
    ecs::system::SystemParam,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    content::{
        biome::BiomeRegistry, dimension::DimensionDefinition, structure::StructureRegistry,
    },
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::chunk_coord_from_position,
        deduplicated_queue::DeduplicatedQueue,
        lighting::PendingLightingUpdates,
        world::VoxelWorld,
    },
};

use self::{
    generation::{collect_generated_chunks, dispatch_generation_tasks},
    meshing::{collect_built_chunk_meshes, dispatch_initial_mesh_tasks},
    selection::rebuild_queue,
};
use super::{
    biome_field::BiomeField,
    chunk_generation_tasks::ChunkGenerationTasks,
    chunk_mesh_tasks::ChunkMeshTasks,
    chunk_remesh::ChunkRemeshQueue,
    chunk_rendering::ChunkRenderPool,
    chunk_system_params::{ChunkContent, ChunkGeneration, ChunkRenderer},
    fluid_updates::{GeneratedFluidPriming, PendingFluidUpdates},
    render_distance::RenderDistanceSettings,
    tick::WorldTickClock,
    world_feature_fields::WorldFeatureFields,
};

const CRITICAL_PLAYER_RADIUS_CHUNKS: i32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CriticalPendingScanKey {
    queue_revision: u64,
    center: IVec3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RetiredScanKey {
    queue_revision: u64,
    selection_revision: u64,
    center: IVec2,
    radius_squared: i32,
}

#[derive(Resource, Default)]
pub(super) struct ChunkStreamingState {
    center: Option<IVec3>,
    movement_direction: IVec2,
    horizontal_radius: i32,
    vertical_radius: i32,
    desired: HashSet<IVec3>,
    retained: HashSet<IVec3>,
    retired: DeduplicatedQueue<IVec3>,
    pending: DeduplicatedQueue<IVec3>,
    ready: DeduplicatedQueue<IVec3>,
    surface_ranges: HashMap<IVec2, (i32, i32)>,
    initial_lighting_seeded: HashSet<IVec3>,
    initial_mesh_seed_catchup: HashSet<IVec3>,
    fluid_priming: GeneratedFluidPriming,
    selection_revision: u64,
    pending_critical_scan_miss: Option<CriticalPendingScanKey>,
    retired_scan_miss: Option<RetiredScanKey>,
}

impl ChunkStreamingState {
    pub(super) fn keeps_loaded(&self, coord: IVec3) -> bool {
        self.desired.contains(&coord) || self.retained.contains(&coord)
    }

    pub(super) fn enqueue_retired(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.retired.enqueue(coord);
        }
    }

    pub(super) fn pop_retired_outside_horizontal_radius(
        &mut self,
        center: IVec3,
        horizontal_radius: i32,
    ) -> Option<IVec3> {
        let center = center.xz();
        let radius_squared = horizontal_radius.max(0).pow(2);
        let scan_key = RetiredScanKey {
            queue_revision: self.retired.revision(),
            selection_revision: self.selection_revision,
            center,
            radius_squared,
        };
        if self.retired_scan_miss == Some(scan_key) {
            return None;
        }

        let desired = &self.desired;
        let retained = &self.retained;
        let coord = self.retired.pop_where(|coord| {
            if desired.contains(&coord) || retained.contains(&coord) {
                return false;
            }

            let delta = coord.xz() - center;
            delta.length_squared() > radius_squared
        });
        if coord.is_some() {
            self.retired_scan_miss = None;
        } else {
            self.retired_scan_miss = Some(scan_key);
        }
        coord
    }

    fn requeue(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.pending.contains(coord) && !self.ready.contains(coord) {
            self.pending.enqueue_front(coord);
        }
    }

    fn defer_pending(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.pending.contains(coord) && !self.ready.contains(coord) {
            self.pending.enqueue(coord);
        }
    }

    fn pop_critical_pending(&mut self) -> Option<IVec3> {
        let center = self.center?;
        let scan_key = CriticalPendingScanKey {
            queue_revision: self.pending.revision(),
            center,
        };
        if self.pending_critical_scan_miss == Some(scan_key) {
            return None;
        }

        let coord = self
            .pending
            .pop_where(|coord| is_critical_streaming_coord(coord, center));
        if coord.is_some() {
            self.pending_critical_scan_miss = None;
        } else {
            self.pending_critical_scan_miss = Some(scan_key);
        }
        coord
    }

    fn mark_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue(coord);
        }
    }

    fn pop_ready(&mut self) -> Option<IVec3> {
        let center = self.center?;
        let movement_direction = self.movement_direction;
        self.ready.pop_min_by_key(|coord| {
            if is_critical_streaming_coord(coord, center) {
                0_u8
            } else if movement_direction != IVec2::ZERO
                && (coord.xz() - center.xz()).dot(movement_direction) > 0
            {
                1
            } else {
                2
            }
        })
    }

    fn defer_ready(&mut self, coord: IVec3) {
        if self.keeps_loaded(coord) && !self.ready.contains(coord) {
            self.ready.enqueue_front(coord);
        }
    }

    fn mark_initial_lighting_seeded(&mut self, coord: IVec3) -> bool {
        self.initial_lighting_seeded.insert(coord)
    }

    pub(super) fn forget_initial_lighting_seeded(&mut self, coord: IVec3) {
        self.initial_lighting_seeded.remove(&coord);
        self.initial_mesh_seed_catchup.remove(&coord);
    }


    fn mark_selection_rebuilt(&mut self) {
        self.selection_revision = self
            .selection_revision
            .checked_add(1)
            .expect("chunk streaming selection revision exhausted");
    }
}

fn is_critical_streaming_coord(coord: IVec3, center: IVec3) -> bool {
    let delta = coord - center;
    delta.x.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
        && delta.y.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
        && delta.z.abs() <= CRITICAL_PLAYER_RADIUS_CHUNKS
}

struct QueueRebuildContext<'a> {
    render_pool: &'a ChunkRenderPool,
    dimension: &'a DimensionDefinition,
    biomes: &'a BiomeRegistry,
    structures: &'a StructureRegistry,
    biome_field: &'a BiomeField,
    feature_fields: &'a WorldFeatureFields,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingWork<'w> {
    world: ResMut<'w, VoxelWorld>,
    state: ResMut<'w, ChunkStreamingState>,
    generation_tasks: ResMut<'w, ChunkGenerationTasks>,
    mesh_tasks: ResMut<'w, ChunkMeshTasks>,
    world_ticks: Res<'w, WorldTickClock>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingSelection<'w, 's> {
    render_distance: Res<'w, RenderDistanceSettings>,
    scratch: Local<'s, selection::QueueRebuildScratch>,
}

#[derive(SystemParam)]
pub(super) struct ChunkStreamingQueues<'w> {
    remesh: ResMut<'w, ChunkRemeshQueue>,
    fluid: ResMut<'w, PendingFluidUpdates>,
    lighting: ResMut<'w, PendingLightingUpdates>,
}

pub(super) fn stream_chunks(
    generation: ChunkGeneration,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    player: Single<&Transform, With<GameplayCamera>>,
    mut selection: ChunkStreamingSelection,
    mut work: ChunkStreamingWork,
    mut queues: ChunkStreamingQueues,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = chunk_coord_from_position(feet_position);
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = selection.render_distance.chunks();
    let vertical_radius = selection.render_distance.vertical_chunks();
    let current_tick = work.world_ticks.current_tick();

    if work.state.center != Some(center)
        || work.state.horizontal_radius != horizontal_radius
        || work.state.vertical_radius != vertical_radius
    {
        let rebuild_context = QueueRebuildContext {
            render_pool: &renderer.pool,
            dimension: generation.dimension(),
            biomes: &content.biomes,
            structures: &generation.structures,
            biome_field: &content.biome_field,
            feature_fields: &generation.feature_fields,
        };
        rebuild_queue(
            &mut work.state,
            center,
            horizontal_radius,
            vertical_radius,
            &mut selection.scratch,
            &rebuild_context,
        );
    }

    work.generation_tasks.sync_snapshot(&generation, &content);
    work.generation_tasks.sync_streaming_region(center);
    work.mesh_tasks.sync_snapshot(&content);

    if work.generation_tasks.pending_count() > 0 || work.state.fluid_priming.is_active() {
        collect_generated_chunks(&content, &mut work, &mut queues, current_tick);
    }
    if work.mesh_tasks.pending_count() > 0 {
        collect_built_chunk_meshes(&content, &mut renderer, &mut work, &mut queues.remesh);
    }
    if work.state.ready.len() > 0 {
        dispatch_initial_mesh_tasks(
            &content,
            &mut renderer,
            &mut work,
            &mut queues,
            current_tick,
        );
    }
    if work.state.pending.len() > 0 {
        dispatch_generation_tasks(
            &content,
            &renderer.pool,
            &mut work,
            &mut queues,
            current_tick,
        );
    }
}

// A resident chunk must never expose unseeded DARK light to a neighboring
// mesh snapshot. This used to happen while generated chunks waited in `ready`
// for a free mesh task slot, darkening whole faces during streaming. Preserve
// the once-per-residency rule and defer convergence through the existing queue.
fn seed_loaded_chunk_lighting(
    coord: IVec3,
    content: &ChunkContent<'_>,
    work: &mut ChunkStreamingWork<'_>,
    queues: &mut ChunkStreamingQueues<'_>,
    current_tick: u64,
) {
    if !work.state.mark_initial_lighting_seeded(coord) {
        return;
    }

    let chunk_is_empty = work
        .world
        .chunk(coord)
        .unwrap_or_else(|| panic!("seeded chunk must be resident: {coord:?}"))
        .is_empty();
    queues.fluid.reactivate_loaded_chunk(coord, current_tick);
    queues.fluid.enqueue_loaded_fluid_frontier(&work.world, coord);
    queues.lighting.seed_chunk_direct_lighting(
        &mut work.world,
        coord,
        content.blocks(),
        content.fluids(),
        content.secondary_properties(),
    );

    // A previously scheduled mesh may have captured a missing halo before
    // this chunk arrived. Reconcile after first publication, never cancel it.
    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if offset == IVec3::ZERO {
                    continue;
                }
                let neighbor = coord + offset;
                if work.mesh_tasks.contains(neighbor) {
                    work.state.initial_mesh_seed_catchup.insert(neighbor);
                }
            }
        }
    }

    if chunk_is_empty {
        queues.lighting.enqueue_empty_chunk_relaxation(coord);
    } else {
        queues.lighting.enqueue_chunk_relaxation(coord);
    }

    // Loading any 16³ section can change direct skylight for every resident
    // section below it in the same x/z column. Minecraft's light engine tracks
    // this through section/column status; Asteria explicitly invalidates the
    // lower resident sections so they converge against the new occluder.
    queues
        .lighting
        .enqueue_loaded_column_below(&work.world, coord);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ready_queue_preserves_critical_forward_background_priority_in_one_scan() {
        let background = IVec3::new(-4, 0, 0);
        let forward = IVec3::new(5, 0, 0);
        let critical = IVec3::new(1, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            movement_direction: IVec2::X,
            ..default()
        };
        state.ready.enqueue(background);
        state.ready.enqueue(forward);
        state.ready.enqueue(critical);

        assert_eq!(state.pop_ready(), Some(critical));
        assert_eq!(state.pop_ready(), Some(forward));
        assert_eq!(state.pop_ready(), Some(background));
        assert_eq!(state.pop_ready(), None);
    }

    #[test]
    fn critical_pending_scan_miss_retries_only_after_queue_or_center_change() {
        let far = IVec3::new(8, 0, 0);
        let critical = IVec3::new(1, 0, 0);
        let mut state = ChunkStreamingState {
            center: Some(IVec3::ZERO),
            ..default()
        };
        state.pending.enqueue(far);

        assert_eq!(state.pop_critical_pending(), None);
        let first_miss = state.pending_critical_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(state.pop_critical_pending(), None);
        assert_eq!(state.pending_critical_scan_miss, first_miss);

        state.pending.enqueue(critical);
        assert_eq!(state.pop_critical_pending(), Some(critical));

        state.center = Some(IVec3::new(8, 0, 0));
        assert_eq!(state.pop_critical_pending(), Some(far));
    }

    #[test]
    fn retired_scan_miss_invalidates_when_selection_rebuilds() {
        let coord = IVec3::new(20, 0, 0);
        let mut state = ChunkStreamingState::default();
        state.enqueue_retired(coord);
        state.retained.insert(coord);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            None
        );
        let first_miss = state.retired_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            None
        );
        assert_eq!(state.retired_scan_miss, first_miss);

        state.retained.remove(&coord);
        state.mark_selection_rebuilt();

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 10),
            Some(coord)
        );
    }

    #[test]
    fn retired_chunks_wait_inside_horizontal_retention_radius() {
        let near = IVec3::new(20, 0, 0);
        let far = IVec3::new(23, 0, 0);
        let mut state = ChunkStreamingState::default();
        state.enqueue_retired(near);
        state.enqueue_retired(far);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            Some(far)
        );
        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            None
        );

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::new(-3, 0, 0), 22),
            Some(near)
        );
    }

    #[test]
    fn retired_chunk_that_reenters_selection_is_not_unloaded() {
        let coord = IVec3::new(30, 0, 0);
        let mut state = ChunkStreamingState::default();
        state.enqueue_retired(coord);
        state.desired.insert(coord);

        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            None
        );

        state.desired.remove(&coord);
        assert_eq!(
            state.pop_retired_outside_horizontal_radius(IVec3::ZERO, 22),
            Some(coord)
        );
    }

    #[test]
    fn initial_lighting_seed_is_once_per_residency_not_per_mesh_retry() {
        let coord = IVec3::new(3, 1, -2);
        let mut state = ChunkStreamingState::default();

        assert!(state.mark_initial_lighting_seeded(coord));
        assert!(!state.mark_initial_lighting_seeded(coord));
        state.initial_mesh_seed_catchup.insert(coord);
        state.forget_initial_lighting_seeded(coord);
        assert!(state.mark_initial_lighting_seeded(coord));
        assert!(!state.initial_mesh_seed_catchup.contains(&coord));
    }


}
