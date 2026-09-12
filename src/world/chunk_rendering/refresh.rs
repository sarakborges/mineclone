use bevy::prelude::*;

use super::{ChunkRenderContext, pool::ChunkRenderPool, spawn::spawn_chunk_mesh};

pub fn refresh_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
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

    // refresh is also the creation path for a resident chunk that has not been
    // rendered yet. This lets streaming defer the only mesh build until after its
    // bounded lighting pass without ever depending on a second streaming frame.
    spawn_chunk_mesh(commands, meshes, render_pool, coord, chunk, context);
}
