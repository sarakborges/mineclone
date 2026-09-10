use bevy::prelude::*;

use crate::voxel::neighbors::CARDINAL_NEIGHBORS;

use super::{ChunkRenderContext, pool::ChunkRenderPool, spawn::spawn_chunk_mesh};

pub fn refresh_adjacent_chunk_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
    for offset in CARDINAL_NEIGHBORS {
        let neighbor = coord + offset;

        if render_pool.contains(neighbor) {
            refresh_chunk_mesh(commands, meshes, render_pool, neighbor, context);
        }
    }
}

pub fn refresh_changed_chunk_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    source_coord: IVec3,
    changed_coords: impl IntoIterator<Item = IVec3>,
    context: &ChunkRenderContext<'_>,
) {
    for coord in changed_coords {
        if coord != source_coord {
            refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
        }
    }
}

pub fn refresh_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
    if !render_pool.contains(coord) {
        return;
    }

    let Some(chunk) = context.world.chunk(coord) else {
        return;
    };

    if let Some((entities, mesh_handles)) = render_pool.take(coord) {
        for entity in entities {
            commands.entity(entity).despawn();
        }
        for handle in mesh_handles {
            let _ = meshes.remove(&handle);
        }
    }

    spawn_chunk_mesh(commands, meshes, render_pool, coord, chunk, context);
}
