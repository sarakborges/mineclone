use std::time::Duration;

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        coordinates::{ChunkCoord, chunk_coord_from_world, chunk_origin},
        world::VoxelWorld,
    },
};

use super::ChunkStreamingState;
use crate::world::{
    chunk_task_queue::{ChunkTaskQueue, CompletedChunkTask},
    fluid_updates::{PendingFluidUpdates, enqueue_generated_fluid_frontier},
    generator::{MaterializedChunk, WorldGenerator},
    revision::TaskInputRevision,
    work_budget::{FrameWorkBudget, WorldFrameWorkBudget},
};

const MAX_MATERIALIZATION_TASKS_IN_FLIGHT: usize = 4;
const MAX_MATERIALIZATION_RESULTS_PER_FRAME: usize = 8;
const MAX_MATERIALIZATION_DISPATCHES_PER_FRAME: usize = 4;
const MATERIALIZATION_RESULT_BUDGET: Duration = Duration::from_millis(1);
const MATERIALIZATION_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const FLUID_SPREAD_TARGETS: [IVec3; 5] =
    [IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
const FRONTIER_SOURCE_NEIGHBORS: [IVec3; 5] =
    [IVec3::Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

#[derive(Resource)]
pub(crate) struct ChunkMaterializationTasks {
    revision: TaskInputRevision,
    tasks: ChunkTaskQueue<MaterializedChunk>,
}

impl Default for ChunkMaterializationTasks {
    fn default() -> Self {
        Self {
            revision: TaskInputRevision::default().next(),
            tasks: ChunkTaskQueue::default(),
        }
    }
}

impl ChunkMaterializationTasks {
    pub(super) fn restart_for_generator_change(&mut self) {
        self.revision = self.revision.next();
        self.tasks = ChunkTaskQueue::default();
    }

    pub(super) const fn revision(&self) -> TaskInputRevision {
        self.revision
    }

    pub(super) fn pending_count(&self) -> usize {
        self.tasks.len()
    }

    pub(super) fn contains(&self, coord: IVec3) -> bool {
        self.tasks.contains(ChunkCoord::from_ivec3(coord))
    }

    fn schedule(&mut self, coord: IVec3, generator: &WorldGenerator) -> bool {
        if self.tasks.len() >= MAX_MATERIALIZATION_TASKS_IN_FLIGHT || self.contains(coord) {
            return false;
        }

        let generator = generator.clone();
        let task =
            AsyncComputeTaskPool::get().spawn(async move { generator.materialize_chunk(coord) });
        self.tasks.insert(
            ChunkCoord::from_ivec3(coord),
            self.revision,
            task,
        )
    }

    fn poll_ready(&mut self) -> Option<CompletedChunkTask<MaterializedChunk>> {
        self.tasks.poll_ready().map(CompletedChunkTask::into_runtime)
    }
}

pub(super) fn collect_materialized_chunks(
    generator_tasks: &mut ChunkMaterializationTasks,
    streaming: &mut ChunkStreamingState,
    world: &mut VoxelWorld,
    pending_fluid: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    current_tick: u64,
    frame_budget: &WorldFrameWorkBudget,
) {
    if generator_tasks.pending_count() == 0 {
        return;
    }

    let current_revision = generator_tasks.revision();
    let mut budget = FrameWorkBudget::new(MATERIALIZATION_RESULT_BUDGET, 1)
        .with_global_deadline(frame_budget.deadline())
        .with_maximum_items(MAX_MATERIALIZATION_RESULTS_PER_FRAME);

    while !budget.exhausted() {
        let Some(completed) = generator_tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        let coord = completed.coord;
        streaming.finish_materializing(coord);
        if completed.revision != current_revision {
            if streaming.keeps_loaded(coord) {
                streaming.enqueue_pending(coord);
            }
            continue;
        }
        if !streaming.keeps_loaded(coord) {
            continue;
        }

        if world.restore_chunk(coord) {
            activate_restored_chunk(world, coord, pending_fluid, fluids, current_tick);
            continue;
        }

        let frontiers = completed
            .output
            .generated_fluid_frontiers()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        world.insert_chunk(coord, completed.output.into_chunk());
        pending_fluid.reactivate_loaded_chunk(coord, current_tick);

        for frontier in frontiers {
            if world.is_loaded_at(frontier.position()) {
                enqueue_generated_fluid_frontier(
                    pending_fluid,
                    fluids,
                    frontier.fluid(),
                    frontier.position(),
                );
            }
        }
        enqueue_neighbor_frontiers_targeting_chunk(world, coord, pending_fluid, fluids);
    }
}

pub(super) fn dispatch_materialization_tasks(
    generator: &WorldGenerator,
    generator_tasks: &mut ChunkMaterializationTasks,
    streaming: &mut ChunkStreamingState,
    world: &mut VoxelWorld,
    pending_fluid: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    current_tick: u64,
    frame_budget: &WorldFrameWorkBudget,
) {
    let mut budget = FrameWorkBudget::new(MATERIALIZATION_DISPATCH_BUDGET, 1)
        .with_global_deadline(frame_budget.deadline())
        .with_maximum_items(MAX_MATERIALIZATION_DISPATCHES_PER_FRAME);

    while generator_tasks.pending_count() < MAX_MATERIALIZATION_TASKS_IN_FLIGHT
        && !budget.exhausted()
    {
        let Some(coord) = streaming.pop_pending_by_priority() else {
            break;
        };
        budget.record(1);

        if !streaming.keeps_loaded(coord) || streaming.is_materializing(coord) {
            continue;
        }
        if world.restore_chunk(coord) {
            activate_restored_chunk(world, coord, pending_fluid, fluids, current_tick);
            continue;
        }

        if generator_tasks.schedule(coord, generator) {
            streaming.mark_materializing(coord);
        } else {
            streaming.enqueue_pending(coord);
            break;
        }
    }
}

fn activate_restored_chunk(
    world: &VoxelWorld,
    coord: IVec3,
    pending_fluid: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
    current_tick: u64,
) {
    pending_fluid.reactivate_loaded_chunk(coord, current_tick);
    enqueue_source_frontiers_to_loaded_targets(world, coord, pending_fluid, fluids);
    enqueue_neighbor_frontiers_targeting_chunk(world, coord, pending_fluid, fluids);
}

fn enqueue_source_frontiers_to_loaded_targets(
    world: &VoxelWorld,
    source_coord: IVec3,
    pending_fluid: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
) {
    let Some(chunk) = world.chunk(source_coord) else {
        return;
    };
    let source_origin = chunk_origin(source_coord);
    chunk.visit_potential_fluid_frontier_sources(|local_source, fluid| {
        let definition = fluids.get(fluid.fluid_id).unwrap_or_else(|| {
            panic!(
                "resident chunk {source_coord:?} references missing fluid id {}",
                fluid.fluid_id
            )
        });
        let source = source_origin + local_source;
        for offset in FLUID_SPREAD_TARGETS {
            let target = source + offset;
            if world.is_loaded_at(target) {
                enqueue_generated_fluid_frontier(
                    pending_fluid,
                    fluids,
                    &definition.id,
                    target,
                );
            }
        }
    });
}

fn enqueue_neighbor_frontiers_targeting_chunk(
    world: &VoxelWorld,
    target_coord: IVec3,
    pending_fluid: &mut PendingFluidUpdates,
    fluids: &FluidRegistry,
) {
    for offset in FRONTIER_SOURCE_NEIGHBORS {
        let source_coord = target_coord + offset;
        let Some(chunk) = world.chunk(source_coord) else {
            continue;
        };
        let source_origin = chunk_origin(source_coord);
        chunk.visit_potential_fluid_frontier_sources(|local_source, fluid| {
            let definition = fluids.get(fluid.fluid_id).unwrap_or_else(|| {
                panic!(
                    "resident chunk {source_coord:?} references missing fluid id {}",
                    fluid.fluid_id
                )
            });
            let source = source_origin + local_source;
            for spread in FLUID_SPREAD_TARGETS {
                let target = source + spread;
                if chunk_coord_from_world(target) == target_coord {
                    enqueue_generated_fluid_frontier(
                        pending_fluid,
                        fluids,
                        &definition.id,
                        target,
                    );
                }
            }
        });
    }
}
