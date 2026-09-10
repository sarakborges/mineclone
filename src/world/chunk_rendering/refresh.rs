use bevy::prelude::*;

use super::{ChunkRenderContext, pool::ChunkRenderPool, spawn::spawn_chunk_mesh};

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
