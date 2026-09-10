use bevy::prelude::*;

use super::{ChunkRenderContext, pool::ChunkRenderPool, spawn::spawn_chunk_mesh};

const CHUNK_NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub fn refresh_adjacent_chunk_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
    for offset in CHUNK_NEIGHBORS {
        let neighbor = coord + offset;

        if render_pool.contains(neighbor) {
            refresh_chunk_mesh(commands, meshes, render_pool, neighbor, context);
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
