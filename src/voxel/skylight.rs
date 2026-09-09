use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use super::{
    chunk::{CHUNK_SIZE, CHUNK_VOLUME},
    world::VoxelWorld,
};

pub(crate) const MAX_SKYLIGHT: u8 = 15;
pub(crate) const LIGHT_UPDATE_RADIUS: i32 = 15;

const NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub(crate) fn initialize_chunk_skylight(world: &mut VoxelWorld, coord: IVec3) {
    if coord.y < 0 || world.chunk(coord).is_none() {
        return;
    }

    let skylight = calculate_chunk_skylight(world, coord);

    if let Some(chunk) = world.chunk_mut(coord) {
        chunk.replace_skylight(skylight);
    }
}

pub(crate) fn relight_around_voxel(
    world: &mut VoxelWorld,
    center: IVec3,
) -> HashSet<IVec3> {
    let mut positions = Vec::new();

    for dy in -LIGHT_UPDATE_RADIUS..=LIGHT_UPDATE_RADIUS {
        for dz in -LIGHT_UPDATE_RADIUS..=LIGHT_UPDATE_RADIUS {
            for dx in -LIGHT_UPDATE_RADIUS..=LIGHT_UPDATE_RADIUS {
                if dx.abs() + dy.abs() + dz.abs() > LIGHT_UPDATE_RADIUS {
                    continue;
                }

                let position = center + IVec3::new(dx, dy, dz);
                if position.y < 0 || !world.is_loaded_at(position) {
                    continue;
                }

                positions.push(position);
            }
        }
    }

    let old_levels = positions
        .iter()
        .copied()
        .map(|position| (position, world.skylight_at(position)))
        .collect::<Vec<_>>();
    let region = positions.iter().copied().collect::<HashSet<_>>();

    for position in &positions {
        world.set_skylight_at(*position, 0);
    }

    let max_loaded_chunk_y = world.highest_loaded_chunk_y();
    let mut highest_solid_by_column = HashMap::<IVec2, i32>::new();
    let mut queue = VecDeque::new();

    for position in positions.iter().copied() {
        if world.is_solid(position) {
            continue;
        }

        let column = IVec2::new(position.x, position.z);
        let highest_solid = *highest_solid_by_column.entry(column).or_insert_with(|| {
            world
                .highest_solid_y_in_column(position.x, position.z, max_loaded_chunk_y)
                .unwrap_or(-1)
        });
        let mut level = if position.y > highest_solid {
            MAX_SKYLIGHT
        } else {
            0
        };

        for direction in NEIGHBORS {
            let neighbor = position + direction;
            if region.contains(&neighbor) {
                continue;
            }

            level = level.max(world.skylight_at(neighbor).saturating_sub(1));
        }

        if level > 0 {
            world.set_skylight_at(position, level);
            queue.push_back(position);
        }
    }

    while let Some(position) = queue.pop_front() {
        let level = world.skylight_at(position);
        if level <= 1 {
            continue;
        }

        let propagated = level - 1;

        for direction in NEIGHBORS {
            let next = position + direction;
            if !region.contains(&next) || world.is_solid(next) {
                continue;
            }
            if propagated <= world.skylight_at(next) {
                continue;
            }

            world.set_skylight_at(next, propagated);
            queue.push_back(next);
        }
    }

    let mut changed_chunks = HashSet::new();

    for (position, old_level) in old_levels {
        if world.skylight_at(position) != old_level {
            changed_chunks.insert(chunk_coord(position));
        }
    }

    changed_chunks
}

fn calculate_chunk_skylight(world: &VoxelWorld, coord: IVec3) -> [u8; CHUNK_VOLUME] {
    let chunk = world
        .chunk(coord)
        .unwrap_or_else(|| panic!("cannot light missing chunk: {coord:?}"));
    let chunk_size = CHUNK_SIZE as i32;
    let origin = coord * chunk_size;
    let max_loaded_chunk_y = world.highest_loaded_chunk_y();
    let mut skylight = [0_u8; CHUNK_VOLUME];
    let mut queue = VecDeque::new();

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = origin.x + x as i32;
            let world_z = origin.z + z as i32;
            let highest_solid = world
                .highest_solid_y_in_column(world_x, world_z, max_loaded_chunk_y)
                .unwrap_or(-1);

            for y in 0..CHUNK_SIZE {
                if chunk.cell_at(x as i32, y as i32, z as i32).is_some() {
                    continue;
                }

                let world_y = origin.y + y as i32;
                if world_y > highest_solid {
                    let local = IVec3::new(x as i32, y as i32, z as i32);
                    let index = local_index(local);
                    skylight[index] = MAX_SKYLIGHT;
                    queue.push_back(local);
                }
            }
        }
    }

    for z in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let local = IVec3::new(x as i32, y as i32, z as i32);
                if chunk.cell_at(local.x, local.y, local.z).is_some() {
                    continue;
                }

                let world_position = origin + local;
                let mut seed = skylight[local_index(local)];

                if x == 0 {
                    seed = seed.max(world.skylight_at(world_position + IVec3::NEG_X).saturating_sub(1));
                }
                if x + 1 == CHUNK_SIZE {
                    seed = seed.max(world.skylight_at(world_position + IVec3::X).saturating_sub(1));
                }
                if y == 0 {
                    seed = seed.max(world.skylight_at(world_position + IVec3::NEG_Y).saturating_sub(1));
                }
                if y + 1 == CHUNK_SIZE {
                    seed = seed.max(world.skylight_at(world_position + IVec3::Y).saturating_sub(1));
                }
                if z == 0 {
                    seed = seed.max(world.skylight_at(world_position + IVec3::NEG_Z).saturating_sub(1));
                }
                if z + 1 == CHUNK_SIZE {
                    seed = seed.max(world.skylight_at(world_position + IVec3::Z).saturating_sub(1));
                }

                let index = local_index(local);
                if seed > skylight[index] {
                    skylight[index] = seed;
                    queue.push_back(local);
                }
            }
        }
    }

    while let Some(local) = queue.pop_front() {
        let level = skylight[local_index(local)];
        if level <= 1 {
            continue;
        }

        let propagated = level - 1;

        for direction in NEIGHBORS {
            let next = local + direction;
            if !inside_chunk(next) || chunk.cell_at(next.x, next.y, next.z).is_some() {
                continue;
            }

            let index = local_index(next);
            if propagated <= skylight[index] {
                continue;
            }

            skylight[index] = propagated;
            queue.push_back(next);
        }
    }

    skylight
}

fn inside_chunk(local: IVec3) -> bool {
    let size = CHUNK_SIZE as i32;
    local.x >= 0
        && local.y >= 0
        && local.z >= 0
        && local.x < size
        && local.y < size
        && local.z < size
}

fn local_index(local: IVec3) -> usize {
    local.x as usize
        + local.z as usize * CHUNK_SIZE
        + local.y as usize * CHUNK_SIZE * CHUNK_SIZE
}

fn chunk_coord(world_position: IVec3) -> IVec3 {
    let size = CHUNK_SIZE as i32;
    IVec3::new(
        world_position.x.div_euclid(size),
        world_position.y.div_euclid(size),
        world_position.z.div_euclid(size),
    )
}
