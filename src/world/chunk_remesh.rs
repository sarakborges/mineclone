use std::time::Duration;

use bevy::prelude::*;

use crate::voxel::{
    deduplicated_queue::DeduplicatedQueue, mesh_snapshot::ChunkMeshSnapshot,
    neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
};

use super::{
    chunk_remesh_tasks::{
        ChunkRemeshTaskKind, ChunkRemeshTaskMeshes, ChunkRemeshTasks,
        MAX_REMESH_TASKS_IN_FLIGHT,
    },
    chunk_rendering::{
        ChunkRenderPool, apply_built_chunk_fluid_meshes, apply_built_chunk_geometry_meshes,
        refresh_chunk_geometry_mesh,
    },
    chunk_system_params::{ChunkContent, ChunkRenderer},
    work_budget::FrameWorkBudget,
};

const REMESH_TASK_DISPATCH_BUDGET: Duration = Duration::from_millis(1);
const REMESH_RESULT_INTEGRATION_BUDGET: Duration = Duration::from_millis(1);
const MAX_REMESH_TASKS_DISPATCHED_PER_FRAME: usize = 2;
const MAX_REMESH_RESULTS_COLLECTED_PER_FRAME: usize = 2;
const MAX_IMMEDIATE_LIGHTING_REMESHES_PER_FRAME: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RenderableScanKey {
    queue_revision: u64,
    pool_revision: u64,
}

#[derive(Resource, Default)]
pub(crate) struct ChunkRemeshQueue {
    queue: DeduplicatedQueue<IVec3>,
    fluid: DeduplicatedQueue<IVec3>,
    immediate_geometry: DeduplicatedQueue<IVec3>,
    immediate_lighting: DeduplicatedQueue<IVec3>,
    geometry_scan_miss: Option<RenderableScanKey>,
    fluid_scan_miss: Option<RenderableScanKey>,
    immediate_geometry_scan_miss: Option<RenderableScanKey>,
    immediate_lighting_scan_miss: Option<RenderableScanKey>,
}

impl ChunkRemeshQueue {
    #[cfg(test)]
    pub(crate) fn enqueue(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.remove(coord);
            self.queue.enqueue(coord);
        }
    }

    pub(crate) fn enqueue_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.remove(coord);
            self.queue.enqueue_front(coord);
        }
    }

    pub(crate) fn enqueue_fluid_priority(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.fluid.enqueue_front(coord);
        }
    }

    fn enqueue_task_priority(&mut self, coord: IVec3, kind: ChunkRemeshTaskKind) {
        match kind {
            ChunkRemeshTaskKind::Geometry => self.enqueue_priority(coord),
            ChunkRemeshTaskKind::Fluid => self.enqueue_fluid_priority(coord),
        }
    }

    pub(crate) fn enqueue_voxel_edit(&mut self, coord: IVec3) {
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue_priority(coord + offset);
        }

        if coord.y >= 0 {
            self.enqueue_priority(coord);
            self.immediate_geometry.enqueue_front(coord);
        }
    }

    pub(crate) fn enqueue_lighting_change(&mut self, coord: IVec3) {
        if coord.y >= 0 {
            self.immediate_lighting.enqueue_front(coord);
        }

        for offset in CARDINAL_NEIGHBORS {
            let neighbor = coord + offset;
            if neighbor.y >= 0 {
                self.immediate_lighting.enqueue(neighbor);
            }
        }
    }

    pub(crate) fn remove(&mut self, coord: IVec3) {
        self.queue.remove(coord);
        self.fluid.remove(coord);
        self.immediate_geometry.remove(coord);
        self.immediate_lighting.remove(coord);
    }

    fn pop_renderable(&mut self, render_pool: &ChunkRenderPool) -> Option<IVec3> {
        let coord = pop_renderable_from(
            &mut self.queue,
            &mut self.geometry_scan_miss,
            render_pool,
        )?;
        self.fluid.remove(coord);
        Some(coord)
    }

    fn pop_renderable_fluid(&mut self, render_pool: &ChunkRenderPool) -> Option<IVec3> {
        pop_renderable_from(&mut self.fluid, &mut self.fluid_scan_miss, render_pool)
    }

    fn pop_renderable_immediate_geometry(
        &mut self,
        render_pool: &ChunkRenderPool,
    ) -> Option<IVec3> {
        let coord = pop_renderable_from(
            &mut self.immediate_geometry,
            &mut self.immediate_geometry_scan_miss,
            render_pool,
        )?;
        self.queue.remove(coord);
        self.fluid.remove(coord);
        Some(coord)
    }

    fn pop_renderable_immediate_lighting(
        &mut self,
        render_pool: &ChunkRenderPool,
    ) -> Option<IVec3> {
        pop_renderable_from(
            &mut self.immediate_lighting,
            &mut self.immediate_lighting_scan_miss,
            render_pool,
        )
    }

    #[cfg(test)]
    fn pop(&mut self) -> Option<IVec3> {
        let coord = self.queue.pop()?;
        self.fluid.remove(coord);
        Some(coord)
    }

    #[cfg(test)]
    fn pop_fluid(&mut self) -> Option<IVec3> {
        self.fluid.pop()
    }

    #[cfg(test)]
    fn pop_immediate_geometry(&mut self) -> Option<IVec3> {
        let coord = self.immediate_geometry.pop()?;
        self.queue.remove(coord);
        self.fluid.remove(coord);
        Some(coord)
    }

    #[cfg(test)]
    fn pop_immediate_lighting(&mut self) -> Option<IVec3> {
        self.immediate_lighting.pop()
    }
}

fn pop_renderable_from(
    queue: &mut DeduplicatedQueue<IVec3>,
    last_miss: &mut Option<RenderableScanKey>,
    render_pool: &ChunkRenderPool,
) -> Option<IVec3> {
    let scan_key = RenderableScanKey {
        queue_revision: queue.revision(),
        pool_revision: render_pool.membership_revision(),
    };
    if *last_miss == Some(scan_key) {
        return None;
    }

    let coord = queue.pop_where(|coord| render_pool.contains(coord));
    if coord.is_some() {
        *last_miss = None;
    } else {
        *last_miss = Some(scan_key);
    }
    coord
}

pub(super) fn process_immediate_geometry_remesh(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
) {
    let Some(coord) = queue.pop_renderable_immediate_geometry(&renderer.pool) else {
        return;
    };
    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );

    refresh_chunk_geometry_mesh(
        &mut renderer.commands,
        &mut renderer.meshes,
        &mut renderer.pool,
        coord,
        &render_context,
    );
}

pub(super) fn process_immediate_lighting_remesh(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
) {
    let Some(first_coord) = queue.pop_renderable_immediate_lighting(&renderer.pool) else {
        return;
    };
    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );

    refresh_chunk_geometry_mesh(
        &mut renderer.commands,
        &mut renderer.meshes,
        &mut renderer.pool,
        first_coord,
        &render_context,
    );

    for _ in 1..MAX_IMMEDIATE_LIGHTING_REMESHES_PER_FRAME {
        let Some(coord) = queue.pop_renderable_immediate_lighting(&renderer.pool) else {
            break;
        };

        refresh_chunk_geometry_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            &render_context,
        );
    }
}

pub(super) fn process_chunk_remesh_queue(
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    world: Res<VoxelWorld>,
    mut queue: ResMut<ChunkRemeshQueue>,
    mut tasks: ResMut<ChunkRemeshTasks>,
) {
    tasks.sync_snapshot(&content);
    collect_completed_remesh_tasks(
        &content,
        &mut renderer,
        &world,
        &mut queue,
        &mut tasks,
    );
    dispatch_remesh_tasks(&world, &renderer.pool, &mut queue, &mut tasks);
}

fn collect_completed_remesh_tasks(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    world: &VoxelWorld,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
) {
    let current_revision = tasks.revision();
    let mut budget = FrameWorkBudget::new(REMESH_RESULT_INTEGRATION_BUDGET, 1)
        .with_maximum_items(MAX_REMESH_RESULTS_COLLECTED_PER_FRAME);

    loop {
        if budget.exhausted() {
            break;
        }

        let Some(completed) = tasks.poll_ready() else {
            break;
        };
        budget.record(1);

        let coord = completed.coord;
        let output = completed.output;
        if !renderer.pool.contains(coord) || world.chunk(coord).is_none() {
            continue;
        }
        if completed.revision != current_revision || !output.dependencies.is_current(world) {
            queue.enqueue_task_priority(coord, output.kind);
            continue;
        }

        let render_context = content.render_context(
            world,
            &renderer.terrain_materials,
            &renderer.fluid_materials,
        );
        match output.meshes {
            ChunkRemeshTaskMeshes::Geometry(meshes) => apply_built_chunk_geometry_meshes(
                &mut renderer.commands,
                &mut renderer.meshes,
                &mut renderer.pool,
                coord,
                meshes,
                &render_context,
            ),
            ChunkRemeshTaskMeshes::Fluid(meshes) => apply_built_chunk_fluid_meshes(
                &mut renderer.commands,
                &mut renderer.meshes,
                &mut renderer.pool,
                coord,
                meshes,
                &render_context,
            ),
        }
    }
}

fn dispatch_remesh_tasks(
    world: &VoxelWorld,
    render_pool: &ChunkRenderPool,
    queue: &mut ChunkRemeshQueue,
    tasks: &mut ChunkRemeshTasks,
) {
    let mut budget = FrameWorkBudget::new(REMESH_TASK_DISPATCH_BUDGET, 1)
        .with_maximum_items(MAX_REMESH_TASKS_DISPATCHED_PER_FRAME);

    while tasks.pending_count() < MAX_REMESH_TASKS_IN_FLIGHT {
        if budget.exhausted() {
            break;
        }

        let next = if let Some(coord) = queue.pop_renderable(render_pool) {
            Some((coord, ChunkRemeshTaskKind::Geometry))
        } else {
            queue
                .pop_renderable_fluid(render_pool)
                .map(|coord| (coord, ChunkRemeshTaskKind::Fluid))
        };
        let Some((coord, kind)) = next else {
            break;
        };

        if tasks.contains(coord) {
            queue.enqueue_task_priority(coord, kind);
            break;
        }
        let Some(snapshot) = ChunkMeshSnapshot::capture(world, coord) else {
            continue;
        };
        if !tasks.schedule(coord, kind, snapshot) {
            queue.enqueue_task_priority(coord, kind);
            break;
        }
        budget.record(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_deduplicates_chunks() {
        let mut queue = ChunkRemeshQueue::default();
        queue.enqueue(IVec3::X);
        queue.enqueue(IVec3::X);

        assert_eq!(queue.pop(), Some(IVec3::X));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn priority_enqueue_moves_existing_chunk_to_front() {
        let mut queue = ChunkRemeshQueue::default();
        queue.enqueue(IVec3::X);
        queue.enqueue(IVec3::Z);
        queue.enqueue_priority(IVec3::Z);

        assert_eq!(queue.pop(), Some(IVec3::Z));
        assert_eq!(queue.pop(), Some(IVec3::X));
    }

    #[test]
    fn full_remesh_supersedes_pending_fluid_remesh() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_fluid_priority(coord);
        queue.enqueue_priority(coord);

        assert_eq!(queue.pop(), Some(coord));
        assert_eq!(queue.pop_fluid(), None);
    }

    #[test]
    fn voxel_edit_keeps_lighting_refresh_separate_from_geometry() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(4, 2, -3);
        queue.enqueue_voxel_edit(coord);

        assert_eq!(queue.pop_immediate_geometry(), Some(coord));
        assert_eq!(queue.pop_immediate_lighting(), None);

        let mut queued = Vec::new();
        while let Some(value) = queue.pop() {
            queued.push(value);
        }
        assert!(!queued.contains(&coord));
        for offset in CARDINAL_NEIGHBORS {
            assert!(queued.contains(&(coord + offset)));
        }
    }

    #[test]
    fn lighting_change_uses_only_stable_lighting_remesh_queue() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(4, 2, -3);
        queue.enqueue_lighting_change(coord);

        let mut lighting = Vec::new();
        while let Some(value) = queue.pop_immediate_lighting() {
            lighting.push(value);
        }

        assert!(lighting.contains(&coord));
        for offset in CARDINAL_NEIGHBORS {
            assert!(lighting.contains(&(coord + offset)));
        }
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn removal_clears_all_pending_remesh_kinds() {
        let mut queue = ChunkRemeshQueue::default();
        let coord = IVec3::new(2, 1, 3);
        queue.enqueue_priority(coord);
        queue.enqueue_fluid_priority(coord);
        queue.immediate_geometry.enqueue(coord);
        queue.immediate_lighting.enqueue(coord);

        queue.remove(coord);

        assert_eq!(queue.pop(), None);
        assert_eq!(queue.pop_fluid(), None);
        assert_eq!(queue.pop_immediate_geometry(), None);
        assert_eq!(queue.pop_immediate_lighting(), None);
    }

    #[test]
    fn renderable_scan_miss_retries_only_after_queue_change() {
        let mut queue = ChunkRemeshQueue::default();
        let render_pool = ChunkRenderPool::default();
        let first = IVec3::new(1, 1, 1);
        let second = IVec3::new(2, 1, 2);
        queue.enqueue(first);

        assert_eq!(queue.pop_renderable(&render_pool), None);
        let first_miss = queue.geometry_scan_miss;
        assert!(first_miss.is_some());

        assert_eq!(queue.pop_renderable(&render_pool), None);
        assert_eq!(queue.geometry_scan_miss, first_miss);

        queue.enqueue(second);
        assert_eq!(queue.pop_renderable(&render_pool), None);
        assert_ne!(queue.geometry_scan_miss, first_miss);
    }
}
