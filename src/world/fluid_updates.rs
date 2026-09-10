use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::voxel::{
    chunk::CHUNK_SIZE,
    fluid::{FluidCell, MAX_FLUID_LEVEL},
    lighting::PendingLightingUpdates,
    neighbors::CARDINAL_NEIGHBORS,
    world::VoxelWorld,
};

use super::chunk_remesh::ChunkRemeshQueue;

const MAX_FLUID_UPDATES_PER_FRAME: usize = 512;
const HORIZONTAL_NEIGHBORS: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

#[derive(Resource, Default)]
pub(crate) struct PendingFluidUpdates {
    pending: VecDeque<IVec3>,
    queued: HashSet<IVec3>,
}

impl PendingFluidUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.enqueue(position);
        for offset in CARDINAL_NEIGHBORS {
            self.enqueue(position + offset);
        }
    }

    fn enqueue(&mut self, position: IVec3) {
        if position.y >= 0 && self.queued.insert(position) {
            self.pending.push_back(position);
        }
    }

    fn pop(&mut self) -> Option<IVec3> {
        let position = self.pending.pop_front()?;
        self.queued.remove(&position);
        Some(position)
    }

    fn clear(&mut self) {
        self.pending.clear();
        self.queued.clear();
    }
}

pub(super) fn clear_fluid_updates(mut pending: ResMut<PendingFluidUpdates>) {
    pending.clear();
}

pub(super) fn process_fluid_updates(
    mut world: ResMut<VoxelWorld>,
    mut pending: ResMut<PendingFluidUpdates>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
) {
    for _ in 0..MAX_FLUID_UPDATES_PER_FRAME {
        let Some(position) = pending.pop() else {
            break;
        };
        if !world.is_loaded_at(position) {
            continue;
        }

        let current = world.fluid_at(position);
        let desired = desired_fluid(&world, position, current);
        if current == desired {
            continue;
        }

        if world.set_fluid_at(position, desired).is_none() {
            continue;
        }

        lighting.enqueue_voxel_edit(position);
        enqueue_remesh(position, &mut remesh_queue);

        pending.enqueue(position);
        for offset in CARDINAL_NEIGHBORS {
            pending.enqueue(position + offset);
        }
    }
}

fn desired_fluid(
    world: &VoxelWorld,
    position: IVec3,
    current: Option<FluidCell>,
) -> Option<FluidCell> {
    if world.is_solid(position) {
        return None;
    }

    if current.is_some_and(FluidCell::is_source) {
        return current;
    }

    if let Some(above) = world.fluid_at(position + IVec3::Y) {
        return Some(FluidCell::flowing(above.fluid_id, MAX_FLUID_LEVEL));
    }

    let below = position - IVec3::Y;
    let supported = position.y == 0 || world.is_solid(below) || world.fluid_at(below).is_some();
    if !supported {
        return None;
    }

    let mut strongest: Option<(u8, crate::content::fluid::FluidId)> = None;

    for offset in HORIZONTAL_NEIGHBORS {
        let Some(neighbor) = world.fluid_at(position + offset) else {
            continue;
        };
        let level = neighbor.level.saturating_sub(1);
        if level == 0 {
            continue;
        }

        let candidate = (level, neighbor.fluid_id);
        if strongest.is_none_or(|current| {
            candidate.0 > current.0 || (candidate.0 == current.0 && candidate.1 < current.1)
        }) {
            strongest = Some(candidate);
        }
    }

    strongest.map(|(level, fluid_id)| FluidCell::flowing(fluid_id, level))
}

fn enqueue_remesh(position: IVec3, remesh_queue: &mut ChunkRemeshQueue) {
    let center = chunk_coord(position);
    remesh_queue.enqueue_priority(center);

    for offset in CARDINAL_NEIGHBORS {
        let neighbor = chunk_coord(position + offset);
        if neighbor != center {
            remesh_queue.enqueue(neighbor);
        }
    }
}

fn chunk_coord(position: IVec3) -> IVec3 {
    let size = CHUNK_SIZE as i32;
    IVec3::new(
        position.x.div_euclid(size),
        position.y.div_euclid(size),
        position.z.div_euclid(size),
    )
}
